//! One transport-owned watch. No publisher thread, event queue, or durable cursor.
use super::*;
use std::io::Write;
use wire::{SessionWatch, WatchBody, WatchCoverage, WatchCursor, WatchRequest};

#[derive(Clone)]
struct ActiveWatch {
    id: String,
    session: String,
    cursor: WatchCursor,
    needs_resync: bool,
}
#[derive(Default)]
pub(super) struct Watch {
    nonce: Option<String>,
    opens: u8,
    active: Option<ActiveWatch>,
}
/// Count/hash without an unbounded serialization buffer. Includes JSON escaping.
struct Budget {
    count: usize,
    hash: Option<Sha256>,
}
impl Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > wire::WATCH_MAX_REPLY - 1 - self.count {
            return Err(io::Error::other("watch_projection_capacity"));
        }
        self.count += bytes.len();
        if let Some(h) = &mut self.hash {
            h.update(bytes);
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Watch {
    pub(super) fn request(
        &mut self,
        broker: &Arc<Broker>,
        endpoint: &str,
        id: u64,
        epoch: &str,
        request: &WatchRequest,
    ) -> Response {
        if epoch != broker.epoch {
            return broker.fail(id, "watch_epoch_stale");
        }
        if !request.valid() {
            return broker.fail(id, "watch_request_invalid");
        }
        if endpoint == "lifecycle" {
            return broker.fail(id, "watch_lifecycle_unsupported");
        }
        let opening = matches!(request, WatchRequest::Open { .. });
        if opening && self.active.is_some() {
            return broker.fail(id, "watch_already_open");
        }
        if opening && self.opens >= wire::WATCH_MAX_OPENS {
            return broker.fail(id, "watch_open_capacity");
        }
        // Identity preparation precedes State/registry locks; identifier is not a grant.
        if opening && self.nonce.is_none() {
            match token() {
                Ok(n) => self.nonce = Some(n),
                Err(_) => return broker.fail(id, "entropy_unavailable"),
            }
        }
        let mut active = if let WatchRequest::Open { session } = request {
            ActiveWatch {
                id: format!("{}-{}", self.nonce.as_ref().unwrap(), self.opens + 1),
                session: session.clone(),
                cursor: WatchCursor {
                    sample: 0,
                    revision: String::new(),
                },
                needs_resync: false,
            }
        } else {
            let Some(a) = &self.active else {
                return broker.fail(id, "watch_unknown");
            };
            if request.subscription() != Some(a.id.as_str()) {
                return broker.fail(id, "watch_unknown");
            }
            a.clone()
        };
        let mut response = broker.response(id);
        let mut frame = SessionWatch {
            version: wire::VERSION.into(),
            profile: wire::SESSION_WATCH_PROFILE.into(),
            epoch: broker.epoch.clone(),
            endpoint: endpoint.into(),
            subscription: active.id.clone(),
            session: active.session.clone(),
            cursor: active.cursor.clone(),
            coverage: WatchCoverage::NoObservation,
            history_complete: false,
            canonical_pending: None,
            degraded: None,
            body: WatchBody::Closed {},
        };
        let closing = matches!(request, WatchRequest::Close { .. });
        let wrong_base = matches!(request,WatchRequest::Next{cursor,..} if active.needs_resync || cursor != &active.cursor);
        if wrong_base {
            frame.body = WatchBody::ResyncRequired {};
            active.needs_resync = true;
        } else if !closing {
            let state = broker.state.lock().unwrap();
            if !state.endpoints.get(endpoint).is_some_and(|e| e.connected) {
                return broker.fail(id, "endpoint_disconnected");
            }
            let Some(value) = state.sessions.get(&active.session) else {
                return broker.fail(id, "unknown_session");
            };
            let Some(sample) = next_sample(active.cursor.sample) else {
                return broker.fail(id, "watch_cursor_exhausted");
            };
            let mut budget = Budget {
                count: 0,
                hash: Some(Sha256::new()),
            };
            budget
                .hash
                .as_mut()
                .unwrap()
                .update(b"namtab.session-watch/0\0");
            let pending = value.pending_output != 0;
            if serde_json::to_writer(&mut budget, &(&value.view, &state.degraded, pending)).is_err()
            {
                return broker.fail(id, "watch_projection_capacity");
            }
            let revision = format!("sha256:{:x}", budget.hash.unwrap().finalize());
            frame.coverage = WatchCoverage::LatestPublishedView;
            frame.canonical_pending = Some(pending);
            frame.degraded = state.degraded.clone();
            frame.cursor = WatchCursor { sample, revision };
            frame.body = if opening || matches!(request, WatchRequest::Resync { .. }) {
                WatchBody::Baseline {
                    view: value.view.clone(),
                }
            } else if frame.cursor.revision == active.cursor.revision {
                WatchBody::Unchanged {}
            } else {
                WatchBody::Replaced {
                    from: active.cursor.clone(),
                    view: value.view.clone(),
                }
            };
            active.cursor = frame.cursor.clone();
            active.needs_resync = false;
        }
        response.watch = Some(frame);
        if serde_json::to_writer(
            &mut Budget {
                count: 0,
                hash: None,
            },
            &response,
        )
        .is_err()
        {
            return broker.fail(id, "watch_projection_capacity");
        }
        // Advance only after the complete bounded response can be formed. A lost
        // response is a cursor gap, never permission to retry or replay input.
        if closing {
            self.active = None;
        } else {
            self.active = Some(active);
            if opening {
                self.opens += 1;
            }
        }
        response
    }
}

fn next_sample(previous: u64) -> Option<u64> {
    previous.checked_add(1).filter(|n| *n <= wire::MAX_ID)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_sample_cannot_wrap_or_exceed_wire_integer_admission() {
        assert_eq!(next_sample(0), Some(1));
        assert_eq!(next_sample(wire::MAX_ID - 1), Some(wire::MAX_ID));
        assert_eq!(next_sample(wire::MAX_ID), None);
        assert_eq!(next_sample(u64::MAX), None);
    }
    #[test]
    fn serialization_budget_counts_escaped_bytes_and_retains_hard_bound() {
        let mut b = Budget {
            count: 0,
            hash: None,
        };
        b.write_all(&vec![b'x'; wire::WATCH_MAX_REPLY - 1]).unwrap();
        assert!(b.write_all(b"x").is_err());
        assert_eq!(b.count, wire::WATCH_MAX_REPLY - 1);
        let mut b = Budget {
            count: 0,
            hash: Some(Sha256::new()),
        };
        assert!(serde_json::to_writer(&mut b, &"\0".repeat(22000)).is_err());
        assert!(b.count < wire::WATCH_MAX_REPLY);
    }
}
