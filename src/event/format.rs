use crate::event::HeaderMap;
use multimap::MultiMap;
use std::convert::Infallible;
pub use tokio_util::bytes::Bytes;

pub trait EventFormat: Sized {
    const CONTENT_TYPE: &str;
    type Error;

    fn try_from_raw(data: &Bytes) -> Result<Self, <Self as EventFormat>::Error>;
}

#[cfg(feature = "json")]
pub type JsonEvent = serde_json::Value;

#[cfg(feature = "json")]
impl EventFormat for JsonEvent {
    const CONTENT_TYPE: &str = "text/event-json";
    type Error = serde_json::Error;
    fn try_from_raw(data: &Bytes) -> Result<Self, <Self as EventFormat>::Error> {
        serde_json::from_slice(data.as_ref())
    }
}

#[derive(Clone, Debug)]
pub struct PlainEvent(pub(crate) HeaderMap, pub(crate) Option<Bytes>);

impl PlainEvent {
    pub fn get_body(&self) -> Option<&Bytes> {
        self.1.as_ref()
    }

    pub fn get_header(&self, header: &str) -> Option<&str> {
        self.0.get_header(header)
    }
}

impl EventFormat for PlainEvent {
    const CONTENT_TYPE: &str = "text/event-plain";
    type Error = Infallible;

    fn try_from_raw(data: &Bytes) -> Result<Self, <Self as EventFormat>::Error> {
        let mut map = MultiMap::new();

        let mut last: usize = 0;
        while data.len() > last
            && let Some(n) = data[last..].iter().position(|c| *c == b'\n')
        {
            let line_end = last + n;
            let line = data.slice(last..line_end);
            if line.is_empty() {
                last = line_end + 1;
                break;
            };
            let split = line.iter().position(|c| *c == b':');
            if let Some(i) = split {
                let k = &line[..i];
                let v = &line[i + 1..]; // Skip the colon
                let b1 = data.slice_ref(k.trim_ascii());
                let b2 = data.slice_ref(v.trim_ascii());
                map.insert(b1, b2);
            }
            if data.len() > line_end + 1 {
                last = line_end + 1;
            } else {
                break;
            }
        }
        let body = data.clone().split_off(last);

        Ok(PlainEvent(
            HeaderMap(map),
            if body.is_empty() { None } else { Some(body) },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    // TODO: cover multi headers

    #[test]
    fn test_plain_event_parsing() {
        let raw_data = indoc! {b"
        Job-UUID: 7f4db78a-17d7-11dd-b7a0-db4edd065621
        Job-Command: originate
        Job-Command-Arg: sofia/default/1005%20'%26park'
        Event-Name: BACKGROUND_JOB
        Core-UUID: 42bdf272-16e6-11dd-b7a0-db4edd065621
        FreeSWITCH-Hostname: ser
        FreeSWITCH-IPv4: 192.168.1.104
        FreeSWITCH-IPv6: 127.0.0.1
        Event-Date-Local: 2008-05-02%2007%3A37%3A03
        Event-Date-GMT: Thu,%2001%20May%202008%2023%3A37%3A03%20GMT
        Event-Date-timestamp: 1209685023894968
        Event-Calling-File: mod_event_socket.c
        Event-Calling-Function: api_exec
        Event-Calling-Line-Number: 609
        Content-Length: 41
        
        +OK 7f4de4bc-17d7-11dd-b7a0-db4edd065621"
        };

        let bytes = Bytes::from_static(raw_data);
        let plain_event = PlainEvent::try_from_raw(&bytes).unwrap();

        // Test header parsing
        assert_eq!(
            plain_event.get_header("Job-UUID"),
            Some("7f4db78a-17d7-11dd-b7a0-db4edd065621")
        );
        assert_eq!(plain_event.get_header("Job-Command"), Some("originate"));
        assert_eq!(
            plain_event.get_header("Job-Command-Arg"),
            Some("sofia/default/1005%20'%26park'")
        );
        assert_eq!(plain_event.get_header("Event-Name"), Some("BACKGROUND_JOB"));
        assert_eq!(
            plain_event.get_header("Core-UUID"),
            Some("42bdf272-16e6-11dd-b7a0-db4edd065621")
        );
        assert_eq!(plain_event.get_header("FreeSWITCH-Hostname"), Some("ser"));
        assert_eq!(
            plain_event.get_header("FreeSWITCH-IPv4"),
            Some("192.168.1.104")
        );
        assert_eq!(plain_event.get_header("FreeSWITCH-IPv6"), Some("127.0.0.1"));
        assert_eq!(
            plain_event.get_header("Event-Calling-File"),
            Some("mod_event_socket.c")
        );
        assert_eq!(
            plain_event.get_header("Event-Calling-Function"),
            Some("api_exec")
        );
        assert_eq!(
            plain_event.get_header("Event-Calling-Line-Number"),
            Some("609")
        );
        assert_eq!(plain_event.get_header("Content-Length"), Some("41"));

        // Test body parsing
        let body = plain_event.get_body().expect("should have body");
        assert_eq!(body.as_ref(), b"+OK 7f4de4bc-17d7-11dd-b7a0-db4edd065621");
    }

    #[test]
    fn test_plain_event_no_body() {
        let raw_data = indoc! { b"
        Event-Name: HEARTBEAT
        Core-UUID: 42bdf272-16e6-11dd-b7a0-db4edd065621
        FreeSWITCH-Hostname: ser
        \n"
        };

        let bytes = Bytes::from_static(raw_data);
        let plain_event = PlainEvent::try_from_raw(&bytes).unwrap();

        assert_eq!(plain_event.get_header("Event-Name"), Some("HEARTBEAT"));
        assert_eq!(
            plain_event.get_header("Core-UUID"),
            Some("42bdf272-16e6-11dd-b7a0-db4edd065621")
        );
        assert_eq!(plain_event.get_header("FreeSWITCH-Hostname"), Some("ser"));

        // Should have no body
        assert!(plain_event.get_body().is_none());
    }

    #[test]
    fn test_plain_event_missing_header() {
        let raw_data = indoc! { b"
        Event-Name: HEARTBEAT
        FreeSWITCH-Hostname: ser
        \n"
        };

        let bytes = Bytes::from_static(raw_data);
        let plain_event = PlainEvent::try_from_raw(&bytes).unwrap();

        assert_eq!(plain_event.get_header("Event-Name"), Some("HEARTBEAT"));
        assert_eq!(plain_event.get_header("NonExistent"), None);
    }

    #[test]
    fn test_plain_event_whitespace_handling() {
        let raw_data = indoc! { b"
        Event-Name:    HEARTBEAT
        Core-UUID:42bdf272-16e6-11dd-b7a0-db4edd065621
        \n"
        };

        let bytes = Bytes::from_static(raw_data);
        let plain_event = PlainEvent::try_from_raw(&bytes).unwrap();

        // Should trim whitespace around values
        assert_eq!(plain_event.get_header("Event-Name"), Some("HEARTBEAT"));
        assert_eq!(
            plain_event.get_header("Core-UUID"),
            Some("42bdf272-16e6-11dd-b7a0-db4edd065621")
        );
    }
}
