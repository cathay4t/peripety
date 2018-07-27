use std::fmt;

use sdjournal::SdJournalError;

#[derive(Debug, Clone)]
pub enum PeripetyError {
    LogSeverityParseError(String),
    ConfError(String),
    JsonSerializeError(String),
    JsonDeserializeError(String),
    NoSupport(String),
    InternalBug(String),
    BlockNoExists(String),
    StorageSubSystemParseError(String),
    InvalidArgument(String),
    LogAccessError(String),
}

impl fmt::Display for PeripetyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                PeripetyError::LogSeverityParseError(ref x)
                | PeripetyError::ConfError(ref x)
                | PeripetyError::JsonSerializeError(ref x)
                | PeripetyError::JsonDeserializeError(ref x)
                | PeripetyError::NoSupport(ref x)
                | PeripetyError::InternalBug(ref x)
                | PeripetyError::BlockNoExists(ref x)
                | PeripetyError::InvalidArgument(ref x)
                | PeripetyError::LogAccessError(ref x)
                | PeripetyError::StorageSubSystemParseError(ref x) => x,
            }
        )
    }
}

impl From<SdJournalError> for PeripetyError {
    fn from(e: SdJournalError) -> Self {
        PeripetyError::LogAccessError(format!("{}", e))
    }
}
