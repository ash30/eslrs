use tokio_util::bytes::Bytes;

use crate::{
    ESLError,
    event::{RawEvent, delegate},
};

#[derive(Clone, Debug)]
pub struct Reply(RawEvent);

impl TryFrom<RawEvent> for Reply {
    type Error = ESLError;
    fn try_from(value: RawEvent) -> Result<Self, Self::Error> {
        if value.is_reply() || value.is_api_response() {
            Ok(Reply(value))
        } else {
            // Internal Logic error!
            Err(ESLError::new(crate::error::ErrorKind::InternalError(
                "Invalid Reply Cast",
            )))
        }
    }
}

impl Reply {
    delegate!(get_header (header: str) -> Option<&str> );
    delegate!(get_body () -> Option<&Bytes> );
    delegate!(get_content_type() -> Option<&str> );

    pub fn is_ok(&self) -> bool {
        match self.get_content_type() {
            Some("command/reply") => self
                .0
                .get_header("Reply-Text")
                .map(|v| v.starts_with("+OK"))
                .unwrap_or_default(),
            Some("api/response") => true,
            _ => false,
        }
    }
}
