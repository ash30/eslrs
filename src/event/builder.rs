use super::format::*;
use std::{borrow::Cow, fmt::Display, marker::PhantomData};

/// Creates an event string to send over esl connection.
///
/// # Examples
/// ```
/// // EventBuilder will set content-type and length
/// // headers for any suitable EventFormat
/// let e = EventBuilder!(
///    "SEND_MESSAGE",
///    "profile" => "internal",
///    "user" => 100;
///
///    serde_json::json!({
///        "content":"Ok"
///    })
/// );
///
/// // PlainText Body
/// let e = EventBuilder!(
///    "SEND_MESSAGE",
///    "profile" => "internal",
///    "user" => 100;
///
///    "Ok"
/// );
/// ```
#[macro_export]
macro_rules! EventBuilder {
    ($name:expr,$($k:expr=>$v:expr),* $(,)?) => {
       format!(concat!("{}\n",$($crate::EventBuilder!(@sub $k)),*, "\n\n"),$name, $($k, $v),*)
    };
    ($name:expr, $($k:expr=>$v:expr),* $(,)? ;$body:expr) => {
        {
            let body = $crate::event::EventBody::from($body);
       format!(
           concat!(
            "{}\n",
            $($crate::EventBuilder!(@sub $k)),*,
            "content-length: {}\n",
            "content-type: {}\n",
            "\n",
            "{}"
            ),
            $name,
            $($k, $v),*,
            body.len(),
            body.content_type(),
            body
        )
        }
    };
    (@sub $n:expr) => {
        "{}: {}\n"
    };
}

pub struct EventBody<'a, T>(Cow<'a, str>, PhantomData<T>);

impl<'a, T> EventBody<'a, T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() > 0
    }
}

impl<'a, T> EventBody<'a, T>
where
    T: EventFormat,
{
    pub fn content_type(&self) -> &'static str {
        T::CONTENT_TYPE
    }
}

impl<'a, T> Display for EventBody<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> From<Json> for EventBody<'a, Json> {
    fn from(value: Json) -> Self {
        EventBody(value.to_string().into(), PhantomData)
    }
}

impl<'a> From<&'a str> for EventBody<'a, PlainText> {
    fn from(value: &'a str) -> Self {
        EventBody(value.into(), PhantomData)
    }
}

impl<'a> From<Cow<'a, str>> for EventBody<'a, PlainText> {
    fn from(value: Cow<'a, str>) -> Self {
        EventBody(value, PhantomData)
    }
}

pub use EventBuilder;
