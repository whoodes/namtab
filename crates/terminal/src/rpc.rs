//! Private, same-build MK1 worker exchange. Not a public/stable ABI or checkpoint.
//! The service imports these data types, never constructs a second Engine.
use crate::{Error, Frame, Input, InputError};
use serde::{Deserialize, Serialize};

pub const MAX_REQUEST_BYTES: usize = 131_072;
pub const MAX_RESPONSE_BYTES: usize = 1_048_576;
pub const MAX_SESSIONS: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// Strictly increasing, nonzero for this private stream. No retries.
    pub id: u64,
    pub op: Operation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", deny_unknown_fields)]
pub enum Operation {
    New {
        session: String,
        cols: u16,
        rows: u16,
    },
    Feed {
        session: String,
        bytes: Vec<u8>,
    },
    Frame {
        session: String,
    },
    Encode {
        session: String,
        input: Input,
    },
    Resize {
        session: String,
        cols: u16,
        rows: u16,
    },
    /// Close the observed output stream without asserting process exit.
    FinishOutput {
        session: String,
    },
    /// Drop only this reducer. This never terminates an application/process.
    Close {
        session: String,
    },
}

impl Operation {
    pub fn session(&self) -> &str {
        match self {
            Self::New { session, .. }
            | Self::Feed { session, .. }
            | Self::Frame { session }
            | Self::Encode { session, .. }
            | Self::Resize { session, .. }
            | Self::Close { session }
            | Self::FinishOutput { session } => session,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub id: u64,
    pub result: Result<Reply, WorkerError>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Reply {
    Frame(Frame),
    Fed {
        frame: Frame,
        replies: Vec<u8>,
    },
    Encoded {
        bytes: Vec<u8>,
        revision: u64,
        mode_revision: u64,
    },
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerError {
    InvalidSession,
    AlreadyExists,
    UnknownSession,
    Capacity,
    Engine(Error),
    Input(InputError),
}
