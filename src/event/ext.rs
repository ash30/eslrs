use super::format::*;

pub trait EventExt {
    fn get_header(&self, header: &str) -> Option<&String>;

    fn get_content_type(&self) -> Option<&str> {
        self.get_header("Content-Type").map(|s| s.as_str())
    }

    fn is_reply(&self) -> bool {
        self.get_header("Reply-Text").is_some()
    }

    fn is_api_response(&self) -> bool {
        matches!(self.get_content_type(), Some("api/response"))
    }

    fn is_json(&self) -> bool {
        self.get_content_type()
            .map(|s| s.starts_with(Json::CONTENT_TYPE))
            .unwrap_or_default()
    }

    fn is_plain_data(&self) -> bool {
        self.get_content_type()
            .map(|s| s.starts_with(PlainEvent::CONTENT_TYPE))
            .unwrap_or_default()
    }
}
