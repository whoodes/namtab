//! Experimental request-driven latest-published session observation, not an event log.
use super::*;
pub const SESSION_WATCH: &str = "session_watch_v0";
pub const SESSION_WATCH_PROFILE: &str = "namtab.session-watch/0";
pub const WATCH_MAX_REPLY: usize = 131_072;
pub const WATCH_MAX_OPENS: u8 = 16;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WatchCursor {
    pub sample: u64,
    pub revision: String,
}
impl WatchCursor {
    pub fn valid(&self) -> bool {
        self.sample > 0 && self.sample <= MAX_ID && super::task_revision_valid(&self.revision)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WatchRequest {
    Open {
        session: String,
    },
    Next {
        subscription: String,
        cursor: WatchCursor,
        credit: u8,
    },
    Resync {
        subscription: String,
    },
    Close {
        subscription: String,
    },
}
impl WatchRequest {
    pub fn valid(&self) -> bool {
        match self {
            Self::Open { session } => ObservationScope::Session {
                session: session.clone(),
            }
            .valid(),
            Self::Next {
                subscription,
                cursor,
                credit,
            } => label(subscription) && cursor.valid() && *credit == 1,
            Self::Resync { subscription } | Self::Close { subscription } => label(subscription),
        }
    }
    pub fn subscription(&self) -> Option<&str> {
        match self {
            Self::Open { .. } => None,
            Self::Next { subscription, .. }
            | Self::Resync { subscription }
            | Self::Close { subscription } => Some(subscription),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WatchCoverage {
    LatestPublishedView,
    NoObservation,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WatchBody {
    Baseline { view: Session },
    Replaced { from: WatchCursor, view: Session },
    Unchanged {},
    ResyncRequired {},
    Closed {},
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SessionWatch {
    pub version: String,
    pub profile: String,
    pub epoch: String,
    pub endpoint: String,
    pub subscription: String,
    pub session: String,
    /// Per-subscription sample ordinal + opaque content revision, not global causality.
    pub cursor: WatchCursor,
    pub coverage: WatchCoverage,
    /// Always false: even an unchanged hash cannot rule out A -> B -> A.
    pub history_complete: bool,
    /// Present only for an actual sample. Pending means not canonically caught up.
    pub canonical_pending: Option<bool>,
    pub degraded: Option<String>,
    pub body: WatchBody,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_read_requests_and_caps_have_no_mutation_name() {
        let cursor = WatchCursor {
            sample: 1,
            revision: format!("sha256:{}", "a".repeat(64)),
        };
        for request in [
            WatchRequest::Open {
                session: "session1".into(),
            },
            WatchRequest::Next {
                subscription: "id".into(),
                cursor: cursor.clone(),
                credit: 1,
            },
            WatchRequest::Resync {
                subscription: "id".into(),
            },
            WatchRequest::Close {
                subscription: "id".into(),
            },
        ] {
            assert!(request.valid());
            let q = Request {
                version: VERSION.into(),
                id: 1,
                operation: Op::SessionWatch {
                    epoch: "a".repeat(64),
                    request,
                },
            };
            assert!(q.operation.mutation_name().is_none());
            assert_eq!(decode(&serde_json::to_vec(&q).unwrap()).unwrap(), q);
        }
        for credit in [0, 2, 255] {
            assert!(
                !WatchRequest::Next {
                    subscription: "id".into(),
                    cursor: cursor.clone(),
                    credit
                }
                .valid()
            );
        }
        assert!(
            !WatchCursor {
                sample: 0,
                revision: cursor.revision.clone()
            }
            .valid()
        );
        assert!(
            !WatchCursor {
                sample: MAX_ID + 1,
                revision: cursor.revision
            }
            .valid()
        );
        assert!(
            !WatchRequest::Open {
                session: "\0".into()
            }
            .valid()
        );
        assert!(
            !WatchRequest::Resync {
                subscription: "x".repeat(81)
            }
            .valid()
        );
    }
}
