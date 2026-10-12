use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PSError {
    NeedMore,
    ParserError,
    StreamIdNotFound,
    InvalidArgument(String),
    InternalError(String),
}

impl fmt::Display for PSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PSError::NeedMore => write!(f, "need more bytes"),
            PSError::ParserError => write!(f, "parser packet error"),
            PSError::StreamIdNotFound => write!(f, "stream id not found"),
            PSError::InvalidArgument(msg) => write!(f, "invalid argument: {}", msg),
            PSError::InternalError(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl std::error::Error for PSError {}

pub type PSResult<T> = Result<T, PSError>;

impl PSError {
    pub fn need_more(&self) -> bool {
        matches!(self, PSError::NeedMore)
    }

    pub fn parser_error(&self) -> bool {
        matches!(self, PSError::ParserError)
    }

    pub fn stream_id_not_found(&self) -> bool {
        matches!(self, PSError::StreamIdNotFound)
    }
}
