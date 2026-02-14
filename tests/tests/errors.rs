// TODO: Maybe should this be merged into other tests???

use std::io;

use specta::Type;
use thiserror::Error;

#[derive(Type, Error, Debug)]
pub enum MyError {
    #[error("data store disconnected")]
    Disconnect(#[specta(type = String)] #[from] io::Error),
    #[error("the data for key `{0}` is not available")]
    Redaction(String),
    #[error("invalid header (expected {expected:?}, found {found:?})")]
    InvalidHeader { expected: String, found: String },
    #[error("unknown data store error")]
    Unknown,
}
