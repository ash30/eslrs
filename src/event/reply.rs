use super::EventExt;
use super::RawEvent;
use crate::ESLError;

#[derive(Clone, Debug)]
pub struct Reply(RawEvent);

impl EventExt for Reply {
    fn get_header(&self, header: &str) -> Option<&String> {
        self.0.get_header(header)
    }
}

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
    pub fn is_ok(&self) -> bool {
        match self.get_content_type() {
            Some("command/reply") => self
                .get_header("Reply-Text")
                .map(|v| v.starts_with("+OK"))
                .unwrap_or_default(),
            Some("api/response") => true,
            _ => false,
        }
    }
}
// ===================
//
