use std::{borrow::Cow, fmt::Arguments};

/// Creates an event string to send over esl connection.
///
/// # Examples
/// ```
/// // EventBuilder will set content-type and length
/// // headers for any suitable EventFormat
/// let e = EventBuilder!(
///    "SEND_MESSAGE",
///    "profile" => "internal",
///    "user" => 100
/// );
///
/// let e = EventBuilder!(
///    "SEND_MESSAGE",
///    "profile" => "internal",
///    "user" => 100;
///
///    "PLAIN TEXT BODY"     
/// );
///
/// let e = EventBuilder!(
///    "SEND_MESSAGE",
///    "profile" => "internal",
///    "user" => 100;
///
///    "OWNED STRING BODY".to_string()
/// );
///
/// let e = EventBuilder!(
///    "SEND_MESSAGE",
///    "profile" => "internal",
///    "user" => 100;
///
///    "Overwriten content type body",
///    "application/simple-message-summary"
/// );
/// ```
#[macro_export]
macro_rules! EventBuilder {
    ($name:expr,$($k:expr=>$v:expr),* $(,)?) => {
        format!("{}\n{}\n",$name, Header!($($k => $v),*))
    };
    ($name:expr, $($k:expr=>$v:expr),* $(,)? $( =>$rest:expr)?;$body:expr $(,$content:expr)?) => {
        format!(
            "{}\n{}{}\n{}",
            $name,
            Header!(
                $($k => $v),*,
                "content-length" => $body.len(),
                "content-type" => $crate::EventBuilder!(@content $($content)?)
            ),
            $crate::EventBuilder!(@rest $($rest)?),
            $body
        )
    };
    (@rest $n:expr) => {
        $n
    };
    (@rest ) => {
        ""
    };
    (@content $n:expr) => {
        $n
    };
    (@content) => {
        "text/plain"
    };
}

#[macro_export]
macro_rules! Header {
    ($($k:expr=>$v:expr),* $(,)?) => {
            format_args!(
               concat!(
                    $($crate::event::Header!(@sub $k)),*,
                ),
                $($k, $v),*
            )
    };
    (@sub $n:expr) => {
        "{}: {}\n"
    };
}

pub(crate) use Header;
