use super::admission::{self, Lease, Pool};
use super::*;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::Ordering;
use std::time::Instant;

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
    let pool = Pool::new(Arc::clone(&broker));
    listener.set_nonblocking(true)?;
    while !broker.stopping.load(Ordering::SeqCst) {
        let Some(lease) = Pool::reserve(&pool) else {
            std::thread::sleep(Duration::from_millis(10));
            continue;
        };
        let stream = match listener.accept() {
            Ok((s, _)) => s,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                drop(lease);
                std::thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        if platform::peer_uid(&stream)? != platform::effective_uid() {
            continue;
        }
        lease.attach(&stream)?;
        let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
        let (reader, endpoint, initial) = if lease.pending() {
            let Ok(read) = stream.try_clone() else {
                continue;
            };
            let mut reader = BufReader::with_capacity(admission::HELLO_BYTES + 1, read);
            let Ok(frame) = classify(&mut reader) else {
                continue;
            };
            let Ok(request) = wire::decode(&frame) else {
                continue;
            };
            // Entropy is prepared without admission/broker locks. A failed
            // candidate may not evict a healthy transport or reserve identity.
            let secret = if matches!(
                request.operation,
                Op::Hello {
                    role: Role::Human,
                    resume: None,
                    ..
                }
            ) {
                token().ok()
            } else {
                None
            };
            let (response, endpoint) = lease.promote(&request, secret);
            if endpoint.is_none() {
                let mut s = stream;
                if let Ok(mut bytes) = serde_json::to_vec(&response) {
                    bytes.push(b'\n');
                    let _ = s.write_all(&bytes);
                }
                continue;
            }
            lease.bind(Role::Human, endpoint.clone());
            (reader, endpoint, Some(response))
        } else {
            (BufReader::new(stream.try_clone()?), None, None)
        };
        let broker = Arc::clone(&broker);
        std::thread::spawn(move || {
            // connection_io owns and drops both I/O handles BEFORE lease/aux
            // release, including unwind. A pending buffered suffix is preserved.
            connection_io(stream, reader, broker, Some(&lease), endpoint, initial);
            drop(lease);
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
/// Absolute saturation-only framing deadline; per-read timeouts alone allow
/// slow trickles forever. Buffer and frame remain bounded, suffix retained.
fn classify(reader: &mut BufReader<UnixStream>) -> io::Result<Vec<u8>> {
    let deadline = Instant::now() + admission::HELLO_TIME;
    let mut frame = Vec::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "admission hello deadline",
            ));
        }
        reader.get_ref().set_read_timeout(Some(remaining))?;
        let bytes = reader.fill_buf()?;
        if bytes.is_empty() {
            return Err(io::Error::other("admission hello EOF"));
        }
        if Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "admission hello deadline",
            ));
        }
        let end = bytes.iter().position(|b| *b == b'\n');
        let n = end.map_or(bytes.len(), |i| i + 1);
        if frame.len() + n > admission::HELLO_BYTES {
            return Err(io::Error::other("admission hello capacity"));
        }
        frame.extend_from_slice(&bytes[..n]);
        reader.consume(n);
        if end.is_some() {
            frame.pop();
            return Ok(frame);
        }
    }
}
#[cfg(test)]
pub(super) fn connection(stream: UnixStream, broker: Arc<Broker>) {
    let reader = BufReader::new(stream.try_clone().unwrap());
    connection_io(stream, reader, broker, None, None, None);
}
fn connection_io(
    mut stream: UnixStream,
    mut reader: BufReader<UnixStream>,
    broker: Arc<Broker>,
    lease: Option<&Lease>,
    mut endpoint: Option<String>,
    mut initial: Option<Response>,
) {
    let mut watch = super::watch::Watch::default();
    let _ = stream.set_read_timeout(Some(Duration::from_secs(300)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    loop {
        let mut accepted_shutdown = false;
        let response = if let Some(response) = initial.take() {
            response
        } else {
            let Ok(Some(frame)) = worker::line(&mut reader, wire::MAX_FRAME) else {
                break;
            };
            let request = wire::decode(&frame);
            let readonly = request.as_ref().is_ok_and(|r| {
                matches!(
                    r.operation,
                    Op::Observe {}
                        | Op::ObserveScoped { .. }
                        | Op::Receipt { .. }
                        | Op::SessionWatch { .. }
                )
            });
            if lease.is_some_and(|l| !l.begin(readonly)) {
                break;
            }
            match request {
                Err(error) => broker.fail(0, error),
                Ok(request) => {
                    if let Some(endpoint) = &endpoint {
                        let stopping = matches!(request.operation, Op::Shutdown {});
                        let id = request.id;
                        let r = if let Op::SessionWatch {
                            epoch,
                            request: selection,
                        } = &request.operation
                        {
                            watch.request(&broker, endpoint, request.id, epoch, selection)
                        } else {
                            broker.request(endpoint, request)
                        };
                        accepted_shutdown = stopping && broker.owns_shutdown(endpoint, id);
                        r
                    } else if matches!(request.operation, Op::LifecycleHello {}) {
                        let (r, bound) = broker.lifecycle_hello(request.id);
                        endpoint = bound;
                        if let Some(l) = lease {
                            l.bind(Role::Human, endpoint.clone());
                        }
                        r
                    } else if let Op::Hello { role, resume, .. } = request.operation {
                        let (r, bound) = broker.hello(request.id, role.clone(), resume);
                        endpoint = bound;
                        if let Some(l) = lease {
                            l.bind(role, endpoint.clone());
                        }
                        r
                    } else {
                        broker.fail(request.id, "hello_required")
                    }
                }
            }
        };
        let Ok(mut bytes) = serde_json::to_vec(&response) else {
            break;
        };
        if bytes.len() > wire::MAX_FRAME {
            break;
        };
        bytes.push(b'\n');
        let written = stream.write_all(&bytes);
        if accepted_shutdown {
            broker.stopping.store(true, Ordering::SeqCst);
            break;
        }
        if written.is_err() {
            break;
        }
        if let Some(l) = lease {
            l.replied();
        }
    }
    // Managed leases perform exactly one disconnect before permit release.
    if lease.is_none()
        && let Some(endpoint) = endpoint
    {
        broker.disconnect(&endpoint);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saturation_classifier_preserves_suffix_and_caps_the_whole_frame() {
        let (mut sender, receiver) = UnixStream::pair().unwrap();
        sender.write_all(b"hello\nnext-frame\n").unwrap();
        let mut reader = BufReader::with_capacity(admission::HELLO_BYTES + 1, receiver);
        assert_eq!(classify(&mut reader).unwrap(), b"hello");
        let mut suffix = String::new();
        reader.read_line(&mut suffix).unwrap();
        assert_eq!(suffix, "next-frame\n");
        let (mut sender, receiver) = UnixStream::pair().unwrap();
        sender
            .write_all(&vec![b'x'; admission::HELLO_BYTES + 1])
            .unwrap();
        assert!(classify(&mut BufReader::new(receiver)).is_err());
    }
    #[test]
    fn trickled_bytes_do_not_renew_the_absolute_classifier_deadline() {
        let (mut sender, receiver) = UnixStream::pair().unwrap();
        let writer = std::thread::spawn(move || {
            for _ in 0..20 {
                if sender.write_all(b" ").is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(40));
            }
        });
        let start = Instant::now();
        assert!(classify(&mut BufReader::new(receiver)).is_err());
        assert!(
            start.elapsed() < Duration::from_millis(700),
            "trickling incorrectly renewed deadline"
        );
        writer.join().unwrap();
    }
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
