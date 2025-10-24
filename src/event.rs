mod builder;
mod ext;
mod format;
mod lazy;
mod reply;

pub use builder::*;
pub use ext::EventExt;
pub use format::*;
pub use reply::Reply;

use crate::connection::RawHeaders;
use lazy::LazyParseCell;
use std::{collections::HashMap, fmt::Debug};

type HeaderMap = HashMap<String, String>;

pub struct Event<T: EventFormat> {
    headers: HeaderMap,
    body: Option<LazyParseCell<T, T::Error>>,
}

impl<T> Debug for Event<T>
where
    T: EventFormat + Debug,
    T::Error: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Event(headers={:?})", self.headers,)
    }
}

impl<T> EventExt for Event<T>
where
    T: EventFormat,
{
    fn get_header(&self, header: &str) -> Option<&String> {
        self.headers.get(header)
    }
}

impl<T> Event<T>
where
    T: EventFormat,
{
    pub(crate) fn new(headers: HeaderMap, body: Option<Bytes>) -> Event<T> {
        Self {
            headers,
            body: body.map(|data| LazyParseCell::new(data)),
        }
    }
}

impl<T> From<RawEvent> for Event<T>
where
    T: EventFormat,
{
    fn from(value: RawEvent) -> Self {
        Event::new(value.headers, value.body)
    }
}

impl Event<Bytes> {
    pub fn cast<T>(self) -> Event<T>
    where
        T: EventFormat,
    {
        debug_assert!(
            self.get_content_type()
                .filter(|a| !a.is_empty())
                .map(|s| s.starts_with(T::CONTENT_TYPE))
                .unwrap_or_default(),
        );
        let data = self.body.map(|cell| cell.get().clone().unwrap());
        Event::new(self.headers, data)
    }
}

// =======

#[derive(Clone, Debug, Default)]
pub(crate) struct RawEvent {
    headers: HeaderMap,
    pub(crate) body: Option<Bytes>,
}

impl RawEvent {
    pub(crate) fn new(headers: RawHeaders, body: Option<Bytes>) -> Self {
        let mut map = HeaderMap::new();
        for h in headers.into_iter() {
            if let Ok(s) = str::from_utf8(&h)
                && let Some((a, b)) = s.split_once(":")
            {
                map.insert(a.trim().to_owned(), b.trim().to_owned());
            }
        }
        Self { body, headers: map }
    }
}

impl EventExt for RawEvent {
    fn get_header(&self, h: &str) -> Option<&String> {
        self.headers.get(h)
    }
}
