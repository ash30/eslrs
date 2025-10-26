use crate::event::Event;
use std::{convert::Infallible, string::FromUtf8Error};

pub trait EventFormat: Sized {
    const CONTENT_TYPE: &str;
    type Error;

    fn try_from_raw(data: &Bytes) -> Result<Self, <Self as EventFormat>::Error>;
}

#[cfg(feature = "json")]
pub type Json = serde_json::Value;

#[cfg(feature = "json")]
impl EventFormat for Json {
    const CONTENT_TYPE: &str = "text/event-json";
    type Error = serde_json::Error;
    fn try_from_raw(data: &Bytes) -> Result<Self, <Self as EventFormat>::Error> {
        serde_json::from_slice(data.as_ref())
    }
}

pub use tokio_util::bytes::Bytes;

impl EventFormat for Bytes {
    const CONTENT_TYPE: &str = "";
    type Error = Infallible;
    fn try_from_raw(data: &Bytes) -> Result<Self, <Self as EventFormat>::Error> {
        Ok(data.clone())
    }
}

pub struct PlainEvent();

impl EventFormat for PlainEvent {
    const CONTENT_TYPE: &str = "text/event-plain";
    type Error = Infallible;
    fn try_from_raw(data: &Bytes) -> Result<Self, <Self as EventFormat>::Error> {
        todo!()
    }
}

// convenience methods to help compiler infer / readability
macro_rules! type_event_helper {
    ($format:ident, $name:ident) => {
        impl Event<$format> {
            pub fn $name(&self) -> Option<&Result<$format, <$format as EventFormat>::Error>> {
                self.body.as_ref().map(|v| v.get())
            }
        }
    };
    ($format:ident, $name:ident, $name2:ident) => {
        type_event_helper!($format, $name);
        impl Event<Bytes> {
            pub fn $name2(self) -> Event<$format> {
                self.cast()
            }
        }
    };
}

type_event_helper!(Bytes, bytes);
type_event_helper!(PlainEvent, data, as_plain_data);
#[cfg(feature = "json")]
type_event_helper!(Json, json, as_json);
