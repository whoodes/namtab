use super::*;
use std::fs;
use std::io::{BufReader, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicUsize, Ordering};

struct SocketGuard {
    path: PathBuf,
    inode: u64,
    device: u64,
}
struct StopGuard(Arc<Broker>);
impl Drop for StopGuard {
    fn drop(&mut self) {
        self.0.shutdown();
    }
}
impl Drop for SocketGuard {
    fn drop(&mut self) {
        if fs::symlink_metadata(&self.path).is_ok_and(|metadata| {
            metadata.ino() == self.inode
                && metadata.dev() == self.device
                && metadata.file_type().is_socket()
        }) {
            let _ = fs::remove_file(&self.path);
        }
    }
}
pub fn serve(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let parent = config.socket.parent().ok_or("socket parent")?;
    if !parent.exists() {
        fs::create_dir(parent)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
    }
    let metadata = fs::symlink_metadata(parent)?;
    if !metadata.is_dir()
        || metadata.uid() != platform::effective_uid()
        || metadata.mode() & 0o777 != 0o700
    {
        return Err("socket directory must be owned mode0700".into());
    }
    if fs::symlink_metadata(&config.socket).is_ok() {
        return Err("socket path exists; refusing to replace or unlink it".into());
    }
    let listener = UnixListener::bind(&config.socket)?;
    fs::set_permissions(&config.socket, fs::Permissions::from_mode(0o600))?;
    let metadata = fs::symlink_metadata(&config.socket)?;
    let _guard = SocketGuard {
        path: config.socket.clone(),
        inode: metadata.ino(),
        device: metadata.dev(),
    };
    let mut journal = config.socket.as_os_str().to_owned();
    journal.push(".journal");
    let broker = Broker::new(config, Path::new(&journal))?;
    let _stop = StopGuard(Arc::clone(&broker));
    let connections = Arc::new(AtomicUsize::new(0));
    listener.set_nonblocking(true)?;
    while !broker.stopping.load(Ordering::SeqCst) {
        let stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        if platform::peer_uid(&stream)? != platform::effective_uid()
            || connections
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                    (n < 4).then_some(n + 1)
                })
                .is_err()
        {
            drop(stream);
            continue;
        }
        let broker = Arc::clone(&broker);
        let connections = Arc::clone(&connections);
        std::thread::spawn(move || {
            connection(stream, broker);
            connections.fetch_sub(1, Ordering::AcqRel);
        });
    }
    // Stop admission first. Drain admitted effects before stopping their worker.
    // Kernel wait may block; a caller timeout is uncertainty, never PID-kill permission.
    while broker.active_mutations.load(Ordering::SeqCst) != 0 {
        std::thread::sleep(Duration::from_millis(10));
    }
    broker.shutdown();
    while broker.active_children.load(Ordering::SeqCst) != 0 {
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}
pub(super) fn connection(mut stream: UnixStream, broker: Arc<Broker>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(300)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let Ok(read) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(read);
    let mut endpoint: Option<String> = None;
    while let Ok(Some(frame)) = worker::line(&mut reader, wire::MAX_FRAME) {
        let mut accepted_shutdown = false;
        let response = match wire::decode(&frame) {
            Err(error) => broker.fail(0, error),
            Ok(request) => {
                if let Some(endpoint) = &endpoint {
                    let stopping = matches!(request.operation, Op::Shutdown {});
                    let id = request.id;
                    let r = broker.request(endpoint, request);
                    accepted_shutdown = stopping && broker.owns_shutdown(endpoint, id);
                    r
                } else if matches!(request.operation, Op::LifecycleHello {}) {
                    let (response, bound) = broker.lifecycle_hello(request.id);
                    endpoint = bound;
                    response
                } else if let Op::Hello { role, resume, .. } = request.operation {
                    let (response, bound) = broker.hello(request.id, role, resume);
                    endpoint = bound;
                    response
                } else {
                    broker.fail(request.id, "hello_required")
                }
            }
        };
        let Ok(mut bytes) = serde_json::to_vec(&response) else {
            break;
        };
        if bytes.len() > wire::MAX_FRAME {
            break;
        }
        bytes.push(b'\n');
        let written = stream.write_all(&bytes);
        // Only the shutdown result writer transitions the listener into cleanup;
        // an ACK lost here does not cancel the already requested stop.
        if accepted_shutdown {
            broker.stopping.store(true, Ordering::SeqCst);
            break;
        }
        if written.is_err() {
            break;
        }
    }
    if let Some(endpoint) = endpoint {
        broker.disconnect(&endpoint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_removes_only_the_same_owned_socket_inode() {
        let directory = std::env::temp_dir().join(format!("mk1-socket-{}", token().unwrap()));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("socket");
        let listener = UnixListener::bind(&path).unwrap();
        let metadata = fs::symlink_metadata(&path).unwrap();
        let guard = SocketGuard {
            path: path.clone(),
            inode: metadata.ino(),
            device: metadata.dev(),
        };
        drop(guard);
        assert!(!path.exists());
        drop(listener);
        let listener = UnixListener::bind(&path).unwrap();
        let metadata = fs::symlink_metadata(&path).unwrap();
        let guard = SocketGuard {
            path: path.clone(),
            inode: metadata.ino(),
            device: metadata.dev(),
        };
        fs::remove_file(&path).unwrap();
        fs::write(&path, "unrelated").unwrap();
        drop(guard);
        assert_eq!(fs::read_to_string(&path).unwrap(), "unrelated");
        drop(listener);
        fs::remove_dir_all(directory).unwrap();
    }
}
