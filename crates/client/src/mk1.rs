//! One bounded synchronous public connection. No mutation is automatically retried.
use project_x_protocol::mk1::{self, Op, Request, Response, Role};
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// Immutable cancellation confirmation target. Not a process handle or a grant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttemptTarget {
    pub epoch: String,
    pub attempt: String,
    pub origin_endpoint: String,
    pub origin_request_id: u64,
}
impl AttemptTarget {
    pub fn observed(s: &mk1::Snapshot, id: &str) -> io::Result<Self> {
        if !s.capabilities.iter().any(|c| c == mk1::ATTEMPT_CANCEL) {
            return Err(io::Error::other(
                "attempt_cancel_v0 unsupported; nothing dispatched",
            ));
        }
        let a = s
            .attempts
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| io::Error::other("selected attempt unavailable"))?;
        let origin = a
            .origin
            .as_ref()
            .ok_or_else(|| io::Error::other("attempt launch origin unknown"))?;
        Ok(Self {
            epoch: s.epoch.clone(),
            attempt: a.id.clone(),
            origin_endpoint: origin.endpoint.clone(),
            origin_request_id: origin.request_id,
        })
    }
}
pub struct Client {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    next: u64,
    pub epoch: String,
    pub endpoint: String,
    pub resume: String,
    failed: bool,
    scoped_support: Option<bool>,
    scoped_work_export_support: bool,
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
            scoped_support: None,
            scoped_work_export_support: false,
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
        Ok(s)
    }
    /// Discover support at most once per transport via legacy Observe. That
    /// bootstrap can refresh artifacts; only the explicitly negotiated WorkExport
    /// scope does so on later pulls. Plain Work/Session/Attempt do not. Never
    /// substitute a partial legacy snapshot or silently fall back on refusal.
    pub fn observe_scoped(
        &mut self,
        scope: mk1::ObservationScope,
    ) -> io::Result<mk1::ScopedObservation> {
        if !scope.valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid observation target; nothing dispatched",
            ));
        }
        if self.scoped_support.is_none() {
            let discovery = self.call(Op::Observe {})?;
            if !discovery.ok
                || discovery.snapshot.is_none()
                || discovery.receipt.is_some()
                || discovery.resume.is_some()
            {
                self.close_transport();
                return Err(io::Error::other(
                    "invalid scoped capability discovery; connection fenced",
                ));
            }
        }
        if self.scoped_support != Some(true) {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "scoped_observe_v0 unsupported; no scoped request dispatched",
            ));
        }
        if matches!(scope, mk1::ObservationScope::WorkExport {}) && !self.scoped_work_export_support
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "scoped_work_export_v0 unsupported; no work export dispatched",
            ));
        }
        let response = self.call(Op::ObserveScoped {
            epoch: self.epoch.clone(),
            scope,
        })?;
        if !response.ok {
            return Err(io::Error::other(
                response
                    .error
                    .unwrap_or_else(|| "scoped observation refused".into()),
            ));
        }
        // call() checked presence, profile and exact requested scope binding.
        Ok(response.observation.expect("validated scoped response"))
    }
    /// Negotiate and re-observe the exact target once. Never retry a lost result.
    pub fn cancel_attempt(&mut self, target: &AttemptTarget) -> io::Result<Response> {
        self.cancel_attempt_guarded(target, || true)
    }
    /// A frontend may withdraw its local intent while negotiation is in flight.
    /// This check precedes dispatch; it is not an atomic OS-focus/permission grant.
    pub fn cancel_attempt_guarded(
        &mut self,
        target: &AttemptTarget,
        current: impl FnOnce() -> bool,
    ) -> io::Result<Response> {
        let s = self.observe_checked()?;
        if AttemptTarget::observed(&s, &target.attempt)? != *target {
            return Err(io::Error::other(
                "cancel target changed; nothing dispatched",
            ));
        }
        if !current() {
            return Err(io::Error::other(
                "local cancellation intent changed; nothing dispatched",
            ));
        }
        let id = self.next_id();
        let r = self.call(Op::CancelAttempt {
            epoch: target.epoch.clone(),
            attempt: target.attempt.clone(),
            origin_endpoint: target.origin_endpoint.clone(),
            origin_request_id: target.origin_request_id,
        })?;
        let receipt_bound = r.receipt.as_ref().is_none_or(|x| {
            x.id == id
                && x.endpoint == self.endpoint
                && x.operation == "cancel_attempt"
                && x.delivered_bytes == 0
        });
        // Error/unknown responses carry the same immutable target binding as
        // success. They are not permission to accept an unrelated typed result.
        let present_result_bound = r.receipt.as_ref().is_none_or(|x| x.result.as_ref().is_none_or(|result| matches!(result, mk1::MutationResult::CancellationRequested {attempt, ..} if attempt == &target.attempt)));
        let result_bound = present_result_bound
            && (!r.ok
                || r.receipt
                    .as_ref()
                    .is_some_and(|x| x.state == "applied" && x.result.is_some()));
        if !receipt_bound
            || !result_bound
            || r.endpoint.as_ref().is_some_and(|e| e != &self.endpoint)
        {
            self.close_transport();
            return Err(io::Error::other(
                "cancel response binding mismatch; effects unknown; no retry",
            ));
        }
        Ok(r)
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
        if let Op::ObserveScoped { epoch, .. } = &operation
            && epoch != &self.epoch
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "scoped request epoch mismatch; nothing dispatched",
            ));
        }
        let scope = match &operation {
            Op::ObserveScoped { scope, .. } => Some(scope.clone()),
            _ => None,
        };
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
            // All consumers share one bound snapshot check, including snapshots
            // attached to mutation/refusal responses. A bad reply cannot authorize
            // another dispatch even when the preceding operation may have acted.
            if let Some(s) = &response.snapshot
                && (s.version != mk1::VERSION
                    || s.epoch != response.epoch
                    || response
                        .endpoint
                        .as_ref()
                        .is_some_and(|ep| ep != &s.endpoint)
                    || (!self.endpoint.is_empty() && s.endpoint != self.endpoint))
            {
                return Err(io::Error::other(
                    "snapshot binding mismatch; effects unknown",
                ));
            }
            match (&scope, &response.observation) {
                (Some(wanted), observation) => {
                    if response.snapshot.is_some()
                        || response.receipt.is_some()
                        || response.resume.is_some()
                        || response
                            .endpoint
                            .as_ref()
                            .is_some_and(|e| e != &self.endpoint)
                        || response.ok != observation.is_some()
                        || (response.ok && response.error.is_some())
                        || observation.as_ref().is_some_and(|o| {
                            o.version != mk1::VERSION
                                || o.profile != mk1::OBSERVATION_PROFILE
                                || o.epoch != self.epoch
                                || o.endpoint != self.endpoint
                                || !o.view.matches(wanted)
                        })
                    {
                        return Err(io::Error::other(
                            "scoped observation binding mismatch; connection fenced",
                        ));
                    }
                }
                (None, Some(_)) => {
                    return Err(io::Error::other(
                        "unrequested scoped observation; connection fenced",
                    ));
                }
                (None, None) => {}
            }
            if let Some(s) = &response.snapshot {
                self.scoped_support = Some(s.capabilities.iter().any(|c| c == mk1::SCOPED_OBSERVE));
                self.scoped_work_export_support =
                    s.capabilities.iter().any(|c| c == mk1::SCOPED_WORK_EXPORT);
            }
            Ok(response)
        })();
        if result.is_err() {
            self.close_transport();
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
