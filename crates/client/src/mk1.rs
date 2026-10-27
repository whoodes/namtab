//! One bounded synchronous public connection. No mutation is automatically retried.
use project_x_protocol::mk1::{self, Op, Request, Response, Role};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

pub struct Client {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    next: u64,
    pub epoch: String,
    pub endpoint: String,
    pub resume: String,
    failed: bool,
}
impl Client {
    pub fn connect(path: &Path, label: &str, role: Role) -> io::Result<Self> {
        Self::connect_resume(path, label, role, None, 1)
    }
    pub fn connect_resume(
        path: &Path,
        label: &str,
        role: Role,
        resume: Option<String>,
        next: u64,
    ) -> io::Result<Self> {
        let mut client = Self::open(path, next)?;
        let response = client.call(Op::Hello {
            label: label.into(),
            role,
            resume,
        })?;
        if !response.ok {
            return Err(io::Error::other(
                response.error.unwrap_or_else(|| "hello refused".into()),
            ));
        }
        client.epoch = response.epoch;
        client.endpoint = response
            .endpoint
            .ok_or_else(|| io::Error::other("missing endpoint"))?;
        client.resume = response
            .resume
            .ok_or_else(|| io::Error::other("missing resume"))?;
        Ok(client)
    }
    fn open(path: &Path, next: u64) -> io::Result<Self> {
        let writer = UnixStream::connect(path)?;
        writer.set_read_timeout(Some(Duration::from_secs(2)))?;
        writer.set_write_timeout(Some(Duration::from_secs(2)))?;
        Ok(Self {
            reader: BufReader::new(writer.try_clone()?),
            writer,
            next,
            epoch: String::new(),
            endpoint: String::new(),
            resume: String::new(),
            failed: false,
        })
    }
    pub fn connect_lifecycle(path: &Path) -> io::Result<Self> {
        let mut c = Self::open(path, 1)?;
        let r = c.call(Op::LifecycleHello {})?;
        if !r.ok {
            return Err(io::Error::other(
                r.error.unwrap_or_else(|| "lifecycle unsupported".into()),
            ));
        }
        c.epoch = r.epoch;
        c.endpoint = r
            .endpoint
            .ok_or_else(|| io::Error::other("missing lifecycle identity"))?;
        Ok(c)
    }
    /// Close the old public transport before an explicit live-owner resume.
    /// This does not undo or retry any operation whose response was lost.
    pub fn close_transport(&mut self) {
        let _ = self.writer.shutdown(std::net::Shutdown::Both);
        self.failed = true;
    }
    pub fn transport_failed(&self) -> bool {
        self.failed
    }
    pub fn observe_checked(&mut self) -> io::Result<mk1::Snapshot> {
        let r = self.call(Op::Observe {})?;
        if !r.ok {
            return Err(io::Error::other(
                r.error.unwrap_or_else(|| "observation refused".into()),
            ));
        }
        let s = r
            .snapshot
            .ok_or_else(|| io::Error::other("missing snapshot"))?;
        if s.version != mk1::VERSION || s.epoch != self.epoch || s.endpoint != self.endpoint {
            self.close_transport();
            return Err(io::Error::other("snapshot binding mismatch"));
        }
        Ok(s)
    }
    pub fn next_id(&self) -> u64 {
        self.next
    }
    pub fn call(&mut self, operation: Op) -> io::Result<Response> {
        if self.failed {
            return Err(io::Error::other(
                "connection lost: effects unknown; reconnect explicitly, never retry input",
            ));
        }
        let id = self.next;
        if id > mk1::MAX_ID {
            return Err(io::Error::other("request IDs exhausted"));
        }
        self.next += 1;
        let request = Request {
            version: mk1::VERSION.into(),
            id,
            operation,
        };
        let mut bytes = serde_json::to_vec(&request)?;
        mk1::decode(&bytes).map_err(io::Error::other)?;
        bytes.push(b'\n');
        let result = (|| {
            self.writer.write_all(&bytes)?;
            let mut bytes = Vec::new();
            let mut limited = self.reader.by_ref().take((mk1::MAX_FRAME + 1) as u64);
            use std::io::Read;
            let n = limited.read_until(b'\n', &mut bytes)?;
            if n == 0 || n > mk1::MAX_FRAME || bytes.last() != Some(&b'\n') {
                return Err(io::Error::other("closed or oversized response"));
            }
            let response: Response = serde_json::from_slice(&bytes)?;
            if response.id != id || (!self.epoch.is_empty() && response.epoch != self.epoch) {
                return Err(io::Error::other(
                    "response identity/epoch mismatch; effects unknown",
                ));
            }
            Ok(response)
        })();
        if result.is_err() {
            self.failed = true;
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    #[test]
    fn missing_socket_is_not_a_mock_connection() {
        assert!(
            Client::connect(Path::new("/nonexistent/namtab.sock"), "test", Role::Agent).is_err()
        );
    }
    #[test]
    fn empty_or_oversize_reply_fences_connection() {
        let dir = std::env::temp_dir().join(format!("namtab-client-test-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("socket");
        let server = UnixListener::bind(&path).unwrap();
        let handle = std::thread::spawn(move || {
            let (mut s, _) = server.accept().unwrap();
            let mut line = String::new();
            BufReader::new(s.try_clone().unwrap())
                .read_line(&mut line)
                .unwrap();
            s.write_all(b"{}\n").unwrap();
        });
        assert!(Client::connect(&path, "test", Role::Agent).is_err());
        handle.join().unwrap();
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
