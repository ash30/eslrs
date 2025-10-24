use crate::event::EventExt;
use crate::{
    Command, ESLError,
    event::{Event, RawEvent, Reply},
};
use futures_util::stream::Fuse;
use futures_util::{Sink, SinkExt, StreamExt, ready};
use pin_project_lite::pin_project;
use std::fmt::Debug;
use std::time::Duration;
use std::{
    collections::VecDeque,
    mem::{self},
    task::{Poll, Waker},
};

#[cfg(feature = "tracing")]
use tracing::{instrument, warn};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_stream::Stream;
use tokio_util::{
    bytes::Bytes,
    codec::{Decoder, Encoder, Framed},
};

pub struct ESLConnection<S> {
    inner: Fuse<ESLConnInner<S>>,
}

impl<S> ESLConnection<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(stream: S) -> Self {
        Self {
            inner: ESLConnInner::new(stream).fuse(),
        }
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), ret, err))]
    pub async fn send_recv<'a, T: Into<Command<'a>> + Debug>(
        &mut self,
        command: T,
    ) -> Result<Reply, ESLError> {
        self.inner.send(command.into()).await?;
        if let Some(event) = self.inner.get_mut().pop_reply() {
            Ok(event.try_into()?)
        } else {
            Err(ESLError::new(crate::error::ErrorKind::InternalError(
                "Missing Sink Reply",
            )))
        }
    }

    #[cfg_attr(feature = "tracing", instrument(skip(self), ret, err))]
    pub async fn recv(&mut self) -> Result<Event<Bytes>, ESLError> {
        if let Some(e) = self.inner.next().await {
            Ok(Event::from(e))
        } else {
            Err(ESLError::new(crate::error::ErrorKind::ConnectionClosed))
        }
    }

    pub async fn disconnect(&mut self) {
        let _ = tokio::time::timeout(
            Duration::from_secs(5),
            self.send_recv(Command::disconnect()),
        )
        .await;
        let _ = self.inner.close().await;
    }

    pub fn is_disconnected(&self) -> bool {
        self.inner.is_done()
    }
}

enum SendRecvState {
    Start,
    Pending(Waker),
    Complete(RawEvent),
}

pin_project! {
    struct ESLConnInner<S> {
        #[pin]
        stream: Framed<S, ESLCodec>,
        active_write: Option<SendRecvState>,
        pending_read: Option<RawHeaders>,
        read_queue: VecDeque<RawEvent>,
    }
}

impl<S> ESLConnInner<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream: Framed::new(stream, ESLCodec::new()),
            pending_read: None,
            active_write: None,
            read_queue: VecDeque::new(),
        }
    }
}

impl<S> ESLConnInner<S>
where
    S: AsyncWrite + AsyncRead + Unpin,
{
    fn pop_reply(&mut self) -> Option<RawEvent> {
        match self.active_write.take() {
            None => None,
            Some(SendRecvState::Complete(e)) => Some(e),
            Some(other) => {
                self.active_write = Some(other);
                None
            }
        }
    }

    fn poll_inner_stream(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<Option<()>, ESLError>> {
        loop {
            let e = match ready!(self.as_mut().project().stream.poll_next(cx)) {
                None => return dbg!(Poll::Ready(Ok(None))),
                Some(Ok(ESLFrame::Header(h))) => {
                    if h.get_header("Content-Length").is_none() {
                        RawEvent::new(h, None)
                    } else {
                        if let Some(h) = self.pending_read.replace(h) {
                            #[cfg(feature = "tracing")]
                            warn!("recv'd new event whilst pending prev body");
                            RawEvent::new(h, None)
                        } else {
                            continue;
                        }
                    }
                }
                Some(Ok(ESLFrame::Body(b))) => {
                    let h = self.pending_read.take().unwrap_or(RawHeaders::new());
                    RawEvent::new(h, b)
                }
                Some(Err(e)) => return Poll::Ready(Err(e)),
            };

            if e.is_reply() || e.is_api_response() {
                if self.active_write.is_some() {
                    match self.active_write.replace(SendRecvState::Complete(e)) {
                        Some(SendRecvState::Pending(w)) => w.wake(),
                        _ => {}
                    }
                } else {
                    // Currently we drop unexpected responses here
                    // but it shouldn't happens since the sink interface
                    // is only SendRecv
                    #[cfg(feature = "tracing")]
                    {
                        let body = e.body.map(|b| {
                            String::from_utf8(b.to_vec()).unwrap_or("parsing error".to_string())
                        });
                        warn!(body, "recv'd unexpected response/reply")
                    }
                }
            } else {
                self.read_queue.push_back(e);
            }
        }
    }
}

impl<'a, S> Sink<Command<'a>> for ESLConnInner<S>
where
    S: AsyncWrite + AsyncRead + Unpin,
{
    type Error = ESLError;

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        let me = self.project();
        //ready!(me.poll_flush(cx))?;
        me.stream.poll_close(cx)
    }
    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        loop {
            match &mut self.active_write {
                None => return Poll::Ready(Ok(())),
                Some(SendRecvState::Start) => {
                    let me = self.as_mut().project();
                    match ready!(me.stream.poll_flush(cx)) {
                        Ok(_) => {
                            *me.active_write = Some(SendRecvState::Pending(cx.waker().clone()));
                            continue;
                        }
                        Err(e) => {
                            // TODO:!!
                            // should we reset the state on error ?
                            return Poll::Ready(Err(e));
                        }
                    }
                }
                Some(SendRecvState::Pending(w)) => {
                    // waker may be overwritten by other reads, so we record it here
                    *w = cx.waker().clone();
                    match ready!(self.as_mut().poll_inner_stream(cx)) {
                        Ok(None) => {
                            return Poll::Ready(Err(ESLError::new(
                                crate::error::ErrorKind::ConnectionClosed,
                            )));
                        } // closed before reply
                        Ok(Some(_)) => {
                            if matches!(self.active_write, Some(SendRecvState::Complete(_))) {
                                continue;
                            } else {
                                return Poll::Pending;
                            };
                        }
                        Err(e) => return Poll::Ready(Err(e)),
                    }
                }
                Some(SendRecvState::Complete(e)) => return Poll::Ready(Ok(())),
            }
        }
    }
    fn start_send(self: std::pin::Pin<&mut Self>, item: Command<'a>) -> Result<(), Self::Error> {
        let me = self.project();
        *me.active_write = Some(SendRecvState::Start);
        me.stream.start_send(item)
    }
    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        // We must be able to 'flush' away any cancelled previous sends
        if self.active_write.is_some() {
            return self.poll_flush(cx);
        }
        self.project().stream.poll_ready(cx)
    }
}

impl<S> Stream for ESLConnInner<S>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    type Item = RawEvent;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        loop {
            match self.as_mut().read_queue.pop_front() {
                None => {
                    // TODO: error
                    match ready!(self.as_mut().poll_inner_stream(cx)) {
                        Ok(None) => return Poll::Ready(None),
                        Ok(Some(())) if self.read_queue.len() > 0 => continue,
                        Ok(Some(())) => return Poll::Pending, // WHY IS THIS RETURNED?
                        Err(e) => continue,                   // TODO: how to handle rrors ..
                    }
                }
                Some(e) => return Poll::Ready(Some(e)),
            }
        }
    }
}

// ==========================

const MAX_HEADERS: usize = 32;
const END: &[u8] = b"\r\n\r\n";

enum ESLFrame {
    Header(RawHeaders),
    Body(Option<Bytes>),
}

pub(crate) struct RawHeaders(Vec<Bytes>);

impl IntoIterator for RawHeaders {
    type Item = Bytes;
    type IntoIter = std::vec::IntoIter<Bytes>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl RawHeaders {
    fn new() -> Self {
        Self(vec![])
    }

    fn append(&mut self, b: Bytes) {
        if self.0.len() < MAX_HEADERS {
            self.0.push(b);
        }
    }
    pub(crate) fn get_header(&self, k: &str) -> Option<&Bytes> {
        self.0.iter().find(|s| s.starts_with(k.as_bytes()))
    }

    pub(crate) fn get_content_len(&self) -> Option<usize> {
        let header = self.get_header("Content-Length")?;
        str::from_utf8(header)
            .map(|s| s.split_once(":"))
            .map(|s| s.and_then(|(_, b)| b.trim().parse::<usize>().ok()))
            .unwrap_or_default()
    }
}

struct ESLCodec {
    decoder: ESLDecoder,
}

impl ESLCodec {
    fn new() -> Self {
        Self {
            decoder: ESLDecoder::PendingHeader {
                headers: RawHeaders::new(),
                current: 0,
            },
        }
    }
}

impl<'a> Encoder<Command<'a>> for ESLCodec {
    type Error = ESLError;
    fn encode(
        &mut self,
        item: Command<'a>,
        dst: &mut tokio_util::bytes::BytesMut,
    ) -> Result<(), Self::Error> {
        let len = item.cmd.len() + item.args.len() + END.len();
        dst.reserve(len);
        dst.extend_from_slice(item.cmd.as_bytes());
        dst.extend_from_slice(item.args.as_bytes());
        dst.extend_from_slice(END);
        Ok(())
    }
}

enum ESLDecoder {
    PendingHeader { headers: RawHeaders, current: usize },
    PendingBody(usize),
}

impl Decoder for ESLCodec {
    type Item = ESLFrame;
    type Error = ESLError;

    fn decode(
        &mut self,
        src: &mut tokio_util::bytes::BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        match &mut self.decoder {
            ESLDecoder::PendingHeader { current, headers } => loop {
                let Some(n) = src[*current..].iter().position(|b| *b == b'\n') else {
                    *current = src.len();
                    return Ok(None);
                };

                // Position of newline in the full buffer
                let newline_pos = *current + n;

                // Extract line (everything up to the newline)
                let line = src.split_to(newline_pos);

                // Consume the newline character
                src.split_to(1);

                *current = 0;

                if line.is_empty() {
                    // Empty line signals end of headers
                    let mut new_headers = RawHeaders::new();
                    mem::swap(headers, &mut new_headers);
                    if let Some(len) = new_headers.get_content_len() {
                        self.decoder = ESLDecoder::PendingBody(len)
                    }
                    return Ok(Some(ESLFrame::Header(new_headers)));
                } else {
                    headers.append(line.freeze());
                }
            },
            ESLDecoder::PendingBody(len) => {
                if src.len() < *len {
                    return Ok(None);
                }
                let buf = src.split_to(*len);
                self.decoder = ESLDecoder::PendingHeader {
                    headers: RawHeaders::new(),
                    current: 0,
                };
                Ok(Some(ESLFrame::Body(Some(buf.freeze()))))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_stream::StreamExt;
    use tokio_test::io::Builder;

    const SESSION_DATA: &[u8] = include_bytes!("../tests/data/freeswitch_session.txt");
    const TEST_DATA_API_RES: &[u8] =
        include_bytes!("../tests/data/multi_command_and_api_response.txt");

    #[tokio::test]
    async fn test_eslconn_basic_framing() {
        let mock_data = b"Content-Type: text/event-plain\n\
                          Event-Name: CUSTOM\n\
                          \n";

        let mock_stream = Builder::new().read(mock_data).build();

        let mut conn = ESLConnInner::new(mock_stream);

        let event = conn.next().await;

        assert!(event.is_some(), "Expected to receive an event");
        let event = event.unwrap();
        assert_eq!(
            event.get_header("Content-Type"),
            Some(&"text/event-plain".to_string())
        );
        assert_eq!(event.get_header("Event-Name"), Some(&"CUSTOM".to_string()));
        assert!(event.body.is_none(), "Expected no body for this event");
    }

    #[tokio::test]
    async fn test_eslconn_basic_framing_multi_event() {
        let mock_data = b"Content-Type: text/event-plain\n\
                          Event-Name: CUSTOM\n\
                          \n\
                          Content-Type: text/event-plain\n\
                          Event-Name: CUSTOM\n\
                          \n";

        let mock_stream = Builder::new().read(mock_data).build();

        let mut conn = ESLConnInner::new(mock_stream);

        for _ in 0..2 {
            let event = conn.next().await;
            assert!(event.is_some(), "Expected to receive an event");
            let event = event.unwrap();
            assert!(event.body.is_none(), "Expected no body for this event");
            assert_eq!(
                event.get_header("Content-Type"),
                Some(&"text/event-plain".to_string())
            );
            assert_eq!(event.get_header("Event-Name"), Some(&"CUSTOM".to_string()));
        }
    }

    #[tokio::test]
    async fn test_events_with_body() {
        let mock_stream = Builder::new().read(TEST_DATA_API_RES).build();
        let mut conn = ESLConnInner::new(mock_stream);

        // Collect all events
        let mut events = Vec::new();
        while let Some(event) = conn.next().await {
            events.push(event);
        }

        // Find events with bodies
        let events_with_body: Vec<_> = events.iter().filter(|e| e.body.is_some()).collect();

        assert!(
            events_with_body.len() >= 1,
            "Expected at least 3 event with bodies"
        );

        // Test plain text BACKGROUND_JOB - it has body with the event details
        let plain_bg_job = events
            .iter()
            .find(|e| {
                e.get_header("Content-Type") == Some(&"text/event-plain".to_string())
                    && e.body.is_some()
                    && e.body.as_ref().map(|b| b.len()) == Some(885)
            })
            .expect("Should find plain text BACKGROUND_JOB event");

        let body_text = std::str::from_utf8(plain_bg_job.body.as_ref().unwrap()).unwrap();
        assert!(body_text.contains("Event-Name: BACKGROUND_JOB"));
        assert!(body_text.contains("Job-Command: status"));

        // Test XML BACKGROUND_JOB
        let xml_bg_job = events
            .iter()
            .find(|e| e.get_header("Content-Type") == Some(&"text/event-xml".to_string()))
            .expect("Should find XML BACKGROUND_JOB event");

        assert!(xml_bg_job.body.is_some());
        let xml_body = std::str::from_utf8(xml_bg_job.body.as_ref().unwrap()).unwrap();
        assert!(xml_body.contains("<Event-Name>BACKGROUND_JOB</Event-Name>"));
        assert!(xml_body.contains("<body>"));

        // Test JSON BACKGROUND_JOB
        let json_bg_job = events
            .iter()
            .find(|e| {
                if let Some(body) = &e.body
                    && let Ok(s) = std::str::from_utf8(body)
                {
                    return s.contains("\"Event-Name\":\"BACKGROUND_JOB\"");
                }
                false
            })
            .expect("Should find JSON BACKGROUND_JOB event");

        let json_body = std::str::from_utf8(json_bg_job.body.as_ref().unwrap()).unwrap();
        assert!(json_body.contains("\"_body\":"));
    }

    #[tokio::test]
    async fn test_all_session_events() {
        let mock_stream = Builder::new().read(SESSION_DATA).build();
        let mut conn = ESLConnInner::new(mock_stream);

        let mut events = Vec::new();
        while let Some(event) = conn.next().await {
            events.push(event);
        }

        // Expected 16 events based on session data analysis
        assert_eq!(
            events.len(),
            16,
            "Expected exactly 16 events from session data"
        );

        // Count events by type
        let command_replies = events
            .iter()
            .filter(|e| e.get_header("Content-Type") == Some(&"command/reply".to_string()))
            .count();
        assert_eq!(command_replies, 9, "Expected 9 command/reply events");

        // Count BACKGROUND_JOB events (they're in the body content)
        let background_jobs = events
            .iter()
            .filter(|e| {
                if let Some(body) = &e.body
                    && let Ok(s) = std::str::from_utf8(body)
                {
                    return s.contains("BACKGROUND_JOB");
                }
                false
            })
            .count();
        assert_eq!(background_jobs, 3, "Expected 3 BACKGROUND_JOB events");

        let heartbeats = events
            .iter()
            .filter(|e| {
                if let Some(body) = &e.body
                    && let Ok(s) = std::str::from_utf8(body)
                {
                    return s.contains("\"Event-Name\":\"HEARTBEAT\"");
                }
                false
            })
            .count();
        assert_eq!(heartbeats, 1, "Expected 1 HEARTBEAT event");

        let events_with_body = events.iter().filter(|e| e.body.is_some()).count();
        assert!(
            events_with_body >= 4,
            "Expected at least 4 events with bodies, got {}",
            events_with_body
        );
    }
}
