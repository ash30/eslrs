mod builder;
mod format;
mod reply;

pub use builder::*;
pub use format::*;
use multimap::MultiMap;
pub use reply::Reply;

use crate::connection::RawHeaders;

#[derive(Clone, Debug)]
pub(crate) struct HeaderMap(MultiMap<Bytes, Bytes>);
impl HeaderMap {
    pub(crate) fn new(headers: RawHeaders) -> Self {
        let mut map = MultiMap::new();
        for h in headers {
            let Some(n) = h.iter().position(|c| *c == b':') else {
                continue;
            };
            let k = h.slice_ref(&h[..n]);
            let v = if h.len() > n {
                h.slice_ref(h[n + 1..].trim_ascii())
            } else {
                Bytes::new()
            };
            map.insert(k, v);
        }
        HeaderMap(map)
    }

    pub fn get_header(&self, k: &str) -> Option<&str> {
        let b = Bytes::copy_from_slice(k.as_ref());
        self.0
            .get(&b)
            .map(|b| str::from_utf8(b).unwrap_or("INVALID UTF8"))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct RawEvent(pub(crate) HeaderMap, pub(crate) Option<Bytes>);

impl RawEvent {
    pub(crate) fn new(headers: RawHeaders, b: Option<Bytes>) -> Self {
        Self(HeaderMap::new(headers), b)
    }
    pub(crate) fn get_header(&self, header: &str) -> Option<&str> {
        self.0.get_header(header)
    }
    pub(crate) fn get_body(&self) -> Option<&Bytes> {
        self.1.as_ref()
    }
    pub(crate) fn get_content_type(&self) -> Option<&str> {
        self.get_header("Content-Type")
    }
    pub(crate) fn is_reply(&self) -> bool {
        self.get_header("Reply-Text").is_some()
    }
    pub(crate) fn is_api_response(&self) -> bool {
        matches!(self.get_content_type(), Some("api/response"))
    }
}

#[derive(Clone, Debug)]
pub struct Event(RawEvent);

impl From<RawEvent> for Event {
    fn from(value: RawEvent) -> Self {
        Event(value)
    }
}

macro_rules! impl_tryfrom {
    ($i:ident) => {
        impl TryFrom<Event> for $i {
            type Error = <$i as EventFormat>::Error;
            fn try_from(value: Event) -> Result<Self, Self::Error> {
                <$i as EventFormat>::try_from_raw(&value.0.get_body().unwrap_or(&Bytes::new()))
            }
        }
    };
}

impl_tryfrom!(PlainEvent);
#[cfg(feature = "json")]
impl_tryfrom!(JsonEvent);

// Delegate Access to RawEvent, as Deref would leak Type
macro_rules! delegate {
    ($f:ident ($($p:ident : $t:path)*) -> $r:path) => {
            pub fn $f(&self, $($p:&$t)*) -> $r {
                self.0.$f($(delegate!(@sub $p $t))*)
        }
    };
    (@sub $p:ident $t:path) => {
        $p
    }
}
pub(crate) use delegate;

impl Event {
    delegate!(get_header (header: str) -> Option<&str> );
    delegate!(get_body () -> Option<&Bytes> );
    delegate!(get_content_type() -> Option<&str> );

    /// Checks if this event has plain text content.
    pub fn is_plain_event(&self) -> bool {
        self.get_content_type()
            .map(|s| s.starts_with(PlainEvent::CONTENT_TYPE))
            .unwrap_or_default()
    }

    /// Checks if this event has JSON content.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use eslrs::event::{Event, JsonEvent};
    /// # fn example(event: Event) {
    /// if event.is_json() {
    ///     // Process as JSON...
    /// }
    /// # }
    /// ```
    #[cfg(feature = "json")]
    pub fn is_json(&self) -> bool {
        self.get_content_type()
            .map(|s| s.starts_with(JsonEvent::CONTENT_TYPE))
            .unwrap_or_default()
    }
}
