//! Strict transport-owned request-driven watches; no automatic credit, resync or replay.
use super::*;
use mk1::{SessionWatch, WatchBody, WatchCoverage, WatchCursor, WatchRequest};
#[derive(Clone)]
pub(super) struct Binding {
    subscription: String,
    session: String,
    cursor: WatchCursor,
    pending: Option<bool>,
    degraded: Option<String>,
    needs_resync: bool,
}
impl Client {
    fn watch_capability(&mut self) -> io::Result<()> {
        if self.watch_support.is_none() {
            self.discover_scoped()?;
        }
        if self.watch_support != Some(true) {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "session_watch_v0 unsupported; nothing dispatched",
            ));
        }
        Ok(())
    }
    pub fn session_watch_open(&mut self, session: &str) -> io::Result<SessionWatch> {
        let request = WatchRequest::Open {
            session: session.into(),
        };
        if !request.valid() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid watch target; nothing dispatched",
            ));
        }
        self.watch_capability()?;
        self.watch_exchange(request)
    }
    /// Credit only after the consumer has applied the previous cursor. An old
    /// application base is a local refusal, never an implicit advance/resync.
    pub fn session_watch_next(&mut self, applied: &SessionWatch) -> io::Result<SessionWatch> {
        let b = self
            .watch_binding
            .as_ref()
            .ok_or_else(|| io::Error::other("no live watch"))?;
        if applied.epoch != self.epoch
            || applied.endpoint != self.endpoint
            || applied.subscription != b.subscription
            || applied.session != b.session
            || applied.coverage != WatchCoverage::LatestPublishedView
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "watch acknowledgement identity mismatch; nothing dispatched",
            ));
        }
        self.watch_exchange(WatchRequest::Next {
            subscription: b.subscription.clone(),
            cursor: applied.cursor.clone(),
            credit: 1,
        })
    }
    pub fn session_watch_resync(&mut self) -> io::Result<SessionWatch> {
        let b = self
            .watch_binding
            .as_ref()
            .ok_or_else(|| io::Error::other("no live watch"))?;
        self.watch_exchange(WatchRequest::Resync {
            subscription: b.subscription.clone(),
        })
    }
    pub fn session_watch_close(&mut self) -> io::Result<SessionWatch> {
        let b = self
            .watch_binding
            .as_ref()
            .ok_or_else(|| io::Error::other("no live watch"))?;
        self.watch_exchange(WatchRequest::Close {
            subscription: b.subscription.clone(),
        })
    }
    fn watch_exchange(&mut self, request: WatchRequest) -> io::Result<SessionWatch> {
        let r = self.call(Op::SessionWatch {
            epoch: self.epoch.clone(),
            request,
        })?;
        if !r.ok {
            return Err(io::Error::other(
                r.error.unwrap_or_else(|| "watch refused".into()),
            ));
        }
        Ok(r.watch.expect("validated watch reply"))
    }
    pub(super) fn watch_preflight(&self, epoch: &str, request: &WatchRequest) -> io::Result<()> {
        let error = if epoch != self.epoch {
            Some("watch epoch mismatch; nothing dispatched")
        } else if !request.valid() {
            Some("invalid watch request; nothing dispatched")
        } else if self.watch_support != Some(true) {
            Some("watch capability not negotiated; nothing dispatched")
        } else if matches!(request, WatchRequest::Open { .. })
            && self.watch_seen.len() >= usize::from(mk1::WATCH_MAX_OPENS)
        {
            Some("watch open budget exhausted; nothing dispatched")
        } else if matches!(request, WatchRequest::Open { .. }) {
            self.watch_binding
                .as_ref()
                .map(|_| "watch already open; nothing dispatched")
        } else {
            match self.watch_binding.as_ref() {
                None => Some("no live watch; nothing dispatched"),
                Some(b) if request.subscription() != Some(b.subscription.as_str()) => {
                    Some("watch transport binding mismatch; nothing dispatched")
                }
                Some(b) if matches!(request,WatchRequest::Next{cursor,..} if b.needs_resync || cursor != &b.cursor) => {
                    Some("watch base stale; explicitly resync, nothing dispatched")
                }
                _ => None,
            }
        };
        error.map_or(Ok(()), |e| {
            Err(io::Error::new(io::ErrorKind::InvalidInput, e))
        })
    }
    pub(super) fn watch_response(
        &mut self,
        wanted: Option<&WatchRequest>,
        r: &Response,
    ) -> io::Result<()> {
        let fail = || io::Error::other("watch response binding/shape mismatch; connection fenced");
        let Some(request) = wanted else {
            return if r.watch.is_none() {
                Ok(())
            } else {
                Err(fail())
            };
        };
        if r.snapshot.is_some()
            || r.observation.is_some()
            || r.receipt.is_some()
            || r.resume.is_some()
            || r.endpoint.as_ref().is_some_and(|e| e != &self.endpoint)
        {
            return Err(fail());
        }
        if !r.ok {
            return if r.watch.is_none() && r.error.as_ref().is_some_and(|e| !e.is_empty()) {
                Ok(())
            } else {
                Err(fail())
            };
        }
        let f = r.watch.as_ref().ok_or_else(fail)?;
        if r.error.is_some()
            || f.version != mk1::VERSION
            || f.profile != mk1::SESSION_WATCH_PROFILE
            || f.epoch != self.epoch
            || f.endpoint != self.endpoint
            || f.history_complete
            || !f.cursor.valid()
            || !(WatchRequest::Resync {
                subscription: f.subscription.clone(),
            })
            .valid()
        {
            return Err(fail());
        }
        if matches!(request, WatchRequest::Open { .. }) && self.watch_seen.contains(&f.subscription)
        {
            return Err(fail());
        }
        let prior = self.watch_binding.as_ref();
        let target = match request {
            WatchRequest::Open { session } => session,
            _ => &prior.ok_or_else(fail)?.session,
        };
        if f.session != *target
            || request
                .subscription()
                .is_some_and(|id| f.subscription != id)
        {
            return Err(fail());
        }
        let sampled = matches!(
            f.body,
            WatchBody::Baseline { .. } | WatchBody::Replaced { .. } | WatchBody::Unchanged {}
        );
        if sampled {
            if f.coverage != WatchCoverage::LatestPublishedView || f.canonical_pending.is_none() {
                return Err(fail());
            }
        } else if f.coverage != WatchCoverage::NoObservation
            || f.canonical_pending.is_some()
            || f.degraded.is_some()
        {
            return Err(fail());
        }
        let next = prior.map_or(Some(1), |b| b.cursor.sample.checked_add(1));
        let valid = match (&f.body, request) {
            (
                WatchBody::Baseline { view },
                WatchRequest::Open { .. } | WatchRequest::Resync { .. },
            ) => view.id == *target && Some(f.cursor.sample) == next,
            (WatchBody::Replaced { from, view }, WatchRequest::Next { cursor, .. }) => {
                view.id == *target
                    && from == cursor
                    && Some(f.cursor.sample) == next
                    && f.cursor.revision != cursor.revision
            }
            (WatchBody::Unchanged {}, WatchRequest::Next { cursor, .. }) => {
                Some(f.cursor.sample) == next
                    && f.cursor.revision == cursor.revision
                    && prior.is_some_and(|b| {
                        f.degraded == b.degraded && f.canonical_pending == b.pending
                    })
            }
            (WatchBody::ResyncRequired {}, WatchRequest::Next { .. })
            | (WatchBody::Closed {}, WatchRequest::Close { .. }) => {
                prior.is_some_and(|b| f.cursor == b.cursor)
            }
            _ => false,
        };
        if !valid {
            return Err(fail());
        }
        // Bounded identifier memory only, not observation/event retention. Close
        // must not allow a corrupt peer to rebrand an old application ack.
        if matches!(request, WatchRequest::Open { .. }) {
            self.watch_seen.push(f.subscription.clone());
        }
        self.watch_binding = if matches!(f.body, WatchBody::Closed {}) {
            None
        } else {
            Some(Binding {
                subscription: f.subscription.clone(),
                session: f.session.clone(),
                cursor: f.cursor.clone(),
                pending: f.canonical_pending,
                degraded: f.degraded.clone(),
                needs_resync: matches!(f.body, WatchBody::ResyncRequired {}),
            })
        };
        Ok(())
    }
}
