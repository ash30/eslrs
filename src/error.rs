use std::{error::Error, fmt::Display};

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ErrorKind {
    /// ESL Connection has closed.
    /// No futher interactions are allowed
    ConnectionClosed,

    /// Error from underlying stream used for esl connection
    /// Generally not recoverable, but you may check error.source()
    /// for more debug info
    IO,

    /// Should never happen, please report via github issue
    InternalError(&'static str),
}

#[derive(Debug)]
pub struct ESLError {
    kind: ErrorKind,
    cause: Option<std::io::Error>,
}

impl ESLError {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self { kind, cause: None }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

impl Display for ESLError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EslError: {:#?}", self.kind)
    }
}

impl Error for ESLError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause.as_ref().map(|e| e as &dyn Error)
    }
}

impl From<std::io::Error> for ESLError {
    fn from(value: std::io::Error) -> Self {
        ESLError {
            kind: ErrorKind::IO,
            cause: Some(value),
        }
    }
}
