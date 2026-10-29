//! Local transport handoff only. Claimed Human is not authenticated human identity.
use super::*;
use std::collections::BTreeMap;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

pub(super) const ADMITTED: usize = 4;
pub(super) const ACCEPTED: usize = 5;
pub(super) const HELLO_BYTES: usize = 1024;
pub(super) const STATE_TIME: Duration = Duration::from_millis(250);
pub(super) const HELLO_TIME: Duration = Duration::from_millis(250);
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Unbound,
    Binding,
    Idle,
    Reading,
    Effect,
    Retired,
    Cleanup,
}
struct Entry {
    admitted: bool,
    phase: Phase,
    role: Option<Role>,
    endpoint: Option<String>,
    socket: Option<UnixStream>,
}
pub(super) struct Pool {
    entries: BTreeMap<u64, Entry>,
    next: u64,
    auxiliary: Option<u64>,
    broker: Arc<Broker>,
}
pub(super) struct Lease {
    pool: Arc<Mutex<Pool>>,
    id: u64,
}
impl Pool {
    pub(super) fn new(broker: Arc<Broker>) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            entries: BTreeMap::new(),
            next: 1,
            auxiliary: None,
            broker,
        }))
    }
    /// Reserve BEFORE accept. A single acceptor can never transiently open #6.
    pub(super) fn reserve(pool: &Arc<Mutex<Self>>) -> Option<Lease> {
        let mut p = pool.lock().unwrap();
        let active = p.entries.values().filter(|e| e.admitted).count();
        let admitted = active < ADMITTED;
        if p.entries.len() >= ACCEPTED || (!admitted && p.auxiliary.is_some()) {
            return None;
        }
        let id = p.next;
        p.next = p.next.checked_add(1)?;
        if !admitted {
            p.auxiliary = Some(id);
        }
        p.entries.insert(
            id,
            Entry {
                admitted,
                phase: if admitted {
                    Phase::Unbound
                } else {
                    Phase::Binding
                },
                role: None,
                endpoint: None,
                socket: None,
            },
        );
        Some(Lease {
            pool: Arc::clone(pool),
            id,
        })
    }
}
impl Lease {
    pub(super) fn attach(&self, stream: &UnixStream) -> io::Result<()> {
        let socket = stream.try_clone()?;
        self.pool
            .lock()
            .unwrap()
            .entries
            .get_mut(&self.id)
            .unwrap()
            .socket = Some(socket);
        Ok(())
    }
    pub(super) fn pending(&self) -> bool {
        !self.pool.lock().unwrap().entries[&self.id].admitted
    }
    pub(super) fn begin(&self, read_only: bool) -> bool {
        let mut p = self.pool.lock().unwrap();
        let e = p.entries.get_mut(&self.id).unwrap();
        if matches!(e.phase, Phase::Retired | Phase::Cleanup) {
            return false;
        }
        e.phase = if e.endpoint.is_none() {
            Phase::Binding
        } else if e.role == Some(Role::Agent) && read_only {
            Phase::Reading
        } else {
            Phase::Effect
        };
        true
    }
    pub(super) fn bind(&self, role: Role, endpoint: Option<String>) {
        let mut p = self.pool.lock().unwrap();
        let e = p.entries.get_mut(&self.id).unwrap();
        e.role = endpoint.as_ref().map(|_| role);
        e.endpoint = endpoint;
    }
    pub(super) fn replied(&self) {
        let mut p = self.pool.lock().unwrap();
        let e = p.entries.get_mut(&self.id).unwrap();
        if !matches!(e.phase, Phase::Retired | Phase::Cleanup) {
            e.phase = if e.endpoint.is_some() {
                Phase::Idle
            } else {
                Phase::Unbound
            };
        }
    }
    /// Acquire broker State before the registry; validation precedes handoff.
    /// No registry mutex is held during bounded State-lock acquisition.
    pub(super) fn promote(
        &self,
        request: &Request,
        secret: Option<String>,
    ) -> (Response, Option<String>) {
        let broker = Arc::clone(&self.pool.lock().unwrap().broker);
        let permit = || {
            let mut p = self.pool.lock().unwrap();
            let victim = p.entries.iter().find_map(|(id, e)| {
                (*id != self.id
                    && e.admitted
                    && e.socket.is_some()
                    && (e.phase == Phase::Unbound
                        || (e.role == Some(Role::Agent)
                            && matches!(e.phase, Phase::Idle | Phase::Reading))))
                .then_some(*id)
            });
            let Some(id) = victim else {
                return false;
            };
            let e = p.entries.get_mut(&id).unwrap();
            e.admitted = false;
            e.phase = Phase::Retired;
            if let Some(socket) = &e.socket {
                let _ = socket.shutdown(Shutdown::Both);
            }
            p.entries.get_mut(&self.id).unwrap().admitted = true;
            p.auxiliary = Some(id);
            true
        };
        match &request.operation {
            Op::Hello {
                role: Role::Human,
                resume,
                ..
            } => broker.control_hello(request.id, resume.clone(), secret, permit),
            Op::LifecycleHello {} => broker.control_lifecycle_hello(request.id, permit),
            _ => (
                broker.fail(
                    request.id,
                    "control_admission_requires_human_or_lifecycle_hello",
                ),
                None,
            ),
        }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        // The handler's reader/writer have already dropped. Keep its permit and
        // shutdown clone until disconnect finishes; never clear a resumed binding
        // in a later cleanup. No registry lock while waiting for broker State.
        let (broker, endpoint) = {
            let mut p = self.pool.lock().unwrap();
            let broker = Arc::clone(&p.broker);
            let e = p.entries.get_mut(&self.id).unwrap();
            e.phase = Phase::Cleanup;
            (broker, e.endpoint.clone())
        };
        if let Some(endpoint) = endpoint {
            broker.disconnect(&endpoint);
        }
        let mut p = self.pool.lock().unwrap();
        p.entries.remove(&self.id);
        if p.auxiliary == Some(self.id) {
            p.auxiliary = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture {
        broker: Arc<Broker>,
        directory: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!("admission-{}", token().unwrap()));
            std::fs::create_dir(&directory).unwrap();
            let target = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .parent()
                .unwrap()
                .to_owned();
            let broker = Broker::new(
                Config {
                    socket: directory.join("socket"),
                    workspace: directory.clone(),
                    worker: target.join("namtab-terminal-worker"),
                    pty_helper: target.join("namtab-pty-helper"),
                    terminfo: None,
                },
                &directory.join("journal"),
            )
            .unwrap();
            Self { broker, directory }
        }
        fn pool(&self) -> Arc<Mutex<Pool>> {
            Pool::new(Arc::clone(&self.broker))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            self.broker.shutdown();
            std::fs::remove_dir_all(&self.directory).unwrap();
        }
    }
    fn hello(resume: Option<String>) -> Request {
        Request {
            version: wire::VERSION.into(),
            id: 1,
            operation: Op::Hello {
                role: Role::Human,
                label: "control".into(),
                resume,
            },
        }
    }
    fn attached(pool: &Arc<Mutex<Pool>>) -> Lease {
        let lease = Pool::reserve(pool).unwrap();
        let (stream, _) = UnixStream::pair().unwrap();
        lease.attach(&stream).unwrap();
        lease
    }
    #[test]
    fn shared_auxiliary_is_not_a_fifth_dispatch_slot_and_retains_cleanup_budget() {
        let f = Fixture::new();
        let p = f.pool();
        let old: Vec<_> = (0..4).map(|_| attached(&p)).collect();
        let candidate = attached(&p);
        assert!(candidate.pending());
        assert!(Pool::reserve(&p).is_none());
        let (r, e) = candidate.promote(&hello(None), Some(token().unwrap()));
        assert!(r.ok);
        candidate.bind(Role::Human, e);
        candidate.replied();
        assert!(
            !old[0].begin(false),
            "retired buffered mutation must not dispatch"
        );
        assert_eq!(
            p.lock()
                .unwrap()
                .entries
                .values()
                .filter(|e| e.admitted)
                .count(),
            4
        );
        assert!(
            Pool::reserve(&p).is_none(),
            "retiring I/O still owns the auxiliary budget"
        );
        drop(old);
        assert!(Pool::reserve(&p).is_some());
    }
    #[test]
    fn mutation_binding_and_human_are_protected_but_reading_agent_is_eligible() {
        let f = Fixture::new();
        let p = f.pool();
        let old: Vec<_> = (0..4).map(|_| attached(&p)).collect();
        old[0].bind(Role::Agent, Some("effect".into()));
        assert!(old[0].begin(false));
        assert!(old[1].begin(true)); // unbound first request = Binding
        old[2].bind(Role::Human, Some("human".into()));
        old[2].replied();
        old[3].bind(Role::Agent, Some("reader".into()));
        assert!(old[3].begin(true));
        let candidate = attached(&p);
        assert!(candidate.promote(&hello(None), Some(token().unwrap())).0.ok);
        assert!(!old[3].begin(false));
        assert!(old[0].begin(false));
        assert!(old[1].begin(false));
        assert!(old[2].begin(false));
        old[3].replied();
        assert!(
            !old[3].begin(true),
            "late completion cannot revive retired handler"
        );
    }
    #[test]
    fn validation_busy_and_identity_exhaustion_never_retire_a_transport() {
        let f = Fixture::new();
        let p = f.pool();
        let old: Vec<_> = (0..4).map(|_| attached(&p)).collect();
        let candidate = attached(&p);
        let mut failures = Vec::new();
        failures.push(candidate.promote(&hello(Some("invalid".into())), None).0);
        failures.push(candidate.promote(&hello(None), None).0);
        {
            let _state = f.broker.state.lock().unwrap();
            failures.push(candidate.promote(&hello(None), Some(token().unwrap())).0);
        }
        let (_, connected) = f.broker.hello(1, Role::Human, None);
        let connected = connected.unwrap();
        let resume = f.broker.state.lock().unwrap().endpoints[&connected]
            .resume
            .clone();
        failures.push(candidate.promote(&hello(Some(resume)), None).0);
        for _ in 1..15 {
            assert!(f.broker.hello(1, Role::Agent, None).0.ok);
        }
        failures.push(candidate.promote(&hello(None), Some(token().unwrap())).0);
        assert_eq!(
            failures
                .iter()
                .map(|r| r.error.as_deref().unwrap())
                .collect::<Vec<_>>(),
            vec![
                "resume_unknown_in_this_epoch",
                "entropy_unavailable",
                "control_admission_state_busy",
                "endpoint_already_connected",
                "endpoint_capacity"
            ]
        );
        for lease in &old {
            assert!(
                lease.begin(false),
                "invalid candidate must not retire an old transport"
            );
        }
    }
    #[test]
    fn no_eligible_transport_is_explicit_and_does_not_allocate_identity() {
        let f = Fixture::new();
        let p = f.pool();
        let old: Vec<_> = (0..4).map(|_| attached(&p)).collect();
        for lease in &old {
            lease.bind(Role::Human, Some("claimed-human".into()));
            lease.replied();
        }
        let candidate = attached(&p);
        let (r, e) = candidate.promote(&hello(None), Some(token().unwrap()));
        assert_eq!(
            r.error.as_deref(),
            Some("control_admission_no_eligible_transport")
        );
        assert!(e.is_none());
        assert!(f.broker.state.lock().unwrap().endpoints.is_empty());
    }
    #[test]
    fn cleanup_disconnects_before_resume_and_never_clears_new_binding() {
        let f = Fixture::new();
        let p = f.pool();
        let lease = attached(&p);
        let (r, e) = f.broker.hello(1, Role::Agent, None);
        let id = e.unwrap();
        let resume = r.resume.unwrap();
        lease.bind(Role::Agent, Some(id.clone()));
        lease.replied();
        drop(lease);
        assert!(!f.broker.state.lock().unwrap().endpoints[&id].connected);
        let replacement = attached(&p);
        let (r, e) = f.broker.hello(2, Role::Agent, Some(resume));
        assert!(r.ok);
        assert_eq!(e.as_ref(), Some(&id));
        replacement.bind(Role::Agent, e);
        replacement.replied();
        assert!(f.broker.state.lock().unwrap().endpoints[&id].connected);
    }
    #[test]
    fn bounded_state_wait_does_not_hold_registry_or_retry_a_handoff() {
        let f = Fixture::new();
        let p = f.pool();
        let old: Vec<_> = (0..4).map(|_| attached(&p)).collect();
        let candidate = attached(&p);
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let b = Arc::clone(&barrier);
        let broker = Arc::clone(&f.broker);
        let pool = Arc::clone(&p);
        let holder = std::thread::spawn(move || {
            let _state = broker.state.lock().unwrap();
            b.wait();
            std::thread::sleep(Duration::from_millis(30));
            // A registry->State lock inversion would deadlock here.
            assert_eq!(pool.lock().unwrap().entries.len(), 5);
        });
        barrier.wait();
        let (r, endpoint) = candidate.promote(&hello(None), Some(token().unwrap()));
        assert!(r.ok, "{r:?}");
        candidate.bind(Role::Human, endpoint);
        holder.join().unwrap();
        assert!(!old[0].begin(false));
        assert_eq!(f.broker.state.lock().unwrap().endpoints.len(), 1);
    }
    #[test]
    fn concurrent_reservations_cannot_exceed_the_preaccept_budget() {
        let f = Fixture::new();
        let p = f.pool();
        let barrier = Arc::new(std::sync::Barrier::new(13));
        let mut threads = Vec::new();
        for _ in 0..12 {
            let pool = Arc::clone(&p);
            let b = Arc::clone(&barrier);
            threads.push(std::thread::spawn(move || {
                let lease = Pool::reserve(&pool);
                b.wait();
                lease
            }));
        }
        barrier.wait();
        let leases: Vec<_> = threads
            .into_iter()
            .filter_map(|t| t.join().unwrap())
            .collect();
        assert_eq!(leases.len(), 5);
        assert_eq!(leases.iter().filter(|l| !l.pending()).count(), 4);
        assert!(Pool::reserve(&p).is_none());
    }
    #[test]
    fn reserved_lifecycle_admission_survives_identity_cap_without_work_authority() {
        let f = Fixture::new();
        let p = f.pool();
        for _ in 0..15 {
            assert!(f.broker.hello(1, Role::Agent, None).0.ok);
        }
        let old: Vec<_> = (0..4).map(|_| attached(&p)).collect();
        let candidate = attached(&p);
        let request = Request {
            version: wire::VERSION.into(),
            id: 1,
            operation: Op::LifecycleHello {},
        };
        let (r, endpoint) = candidate.promote(&request, None);
        assert!(r.ok);
        assert_eq!(endpoint.as_deref(), Some("lifecycle"));
        candidate.bind(Role::Human, endpoint);
        candidate.replied();
        assert!(!old[0].begin(false));
        let denied = f.broker.request(
            "lifecycle",
            Request {
                version: wire::VERSION.into(),
                id: 2,
                operation: Op::CreateSession {
                    name: "forbidden".into(),
                },
            },
        );
        assert_eq!(
            denied.error.as_deref(),
            Some("lifecycle_has_no_work_or_input_authority")
        );
        assert_eq!(f.broker.state.lock().unwrap().endpoints.len(), 16);
    }
}
