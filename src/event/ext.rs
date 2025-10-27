use super::format::*;

pub trait EventExt {
    /// Gets a header value from the event.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use eslrs::event::{Event, EventExt, Bytes};
    /// # fn example(event: Event<Bytes>) {
    /// if let Some(uuid) = event.get_header("Unique-ID") {
    ///     println!("Channel UUID: {}", uuid);
    /// }
    /// # }
    /// ```
    fn get_header(&self, header: &str) -> Option<&String>;

    /// Returns the Content-Type header value.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use eslrs::event::{Event, EventExt, Bytes};
    /// # fn example(event: Event<Bytes>) {
    /// match event.get_content_type() {
    ///     Some("text/event-json") => println!("JSON event"),
    ///     Some("command/reply") => println!("Command reply"),
    ///     _ => println!("Other event type"),
    /// }
    /// # }
    /// ```
    fn get_content_type(&self) -> Option<&str> {
        self.get_header("Content-Type").map(|s| s.as_str())
    }

    /// Checks if this is a command reply event.
    fn is_reply(&self) -> bool {
        self.get_header("Reply-Text").is_some()
    }

    /// Checks if this is an API response event.
    fn is_api_response(&self) -> bool {
        matches!(self.get_content_type(), Some("api/response"))
    }

    /// Checks if this event has JSON content.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use eslrs::event::{Event, EventExt, Bytes};
    /// # fn example(event: Event<Bytes>) {
    /// if event.is_json() {
    ///     let json_event = event.as_json();
    ///     // Process as JSON...
    /// }
    /// # }
    /// ```
    fn is_json(&self) -> bool {
        self.get_content_type()
            .map(|s| s.starts_with(Json::CONTENT_TYPE))
            .unwrap_or_default()
    }

    /// Checks if this event has plain text content.
    fn is_plain_data(&self) -> bool {
        self.get_content_type()
            .map(|s| s.starts_with(PlainEvent::CONTENT_TYPE))
            .unwrap_or_default()
    }
}
