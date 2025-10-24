use crate::event::EventBuilder;
use std::borrow::Cow;

#[derive(Debug)]
pub struct Command<'a> {
    pub(crate) cmd: &'a str,
    pub(crate) args: Cow<'a, str>,
}

impl<'a> From<&'a str> for Command<'a> {
    fn from(value: &'a str) -> Self {
        Command {
            cmd: value,
            args: "".into(),
        }
    }
}

impl<'a, T> From<&'a T> for Command<'a>
where
    T: AsRef<str>,
{
    fn from(value: &'a T) -> Self {
        Command {
            cmd: value.as_ref(),
            args: "".into(),
        }
    }
}

macro_rules! create_command {
    ($name:ident) => {
        create_command!($name, stringify!($name));
    };
    ($name:ident, $cmd:expr) => {
        impl<'a> Command<'a> {
            pub fn $name<T: Into<std::borrow::Cow<'a, str>>>(s: T) -> Command<'a> {
                Command {
                    cmd: concat!($cmd, " "),
                    args: s.into(),
                }
            }
        }
    };
    ($name:ident, $cmd:expr, no_args) => {
        impl<'a> Command<'a> {
            pub fn $name() -> Command<'a> {
                Command {
                    cmd: concat!($cmd, " "),
                    args: "".into(),
                }
            }
        }
    };
}

create_command!(api);
create_command!(sendevent);
create_command!(filter);
create_command!(filter_delete, "filter delete");
create_command!(events, "event plain");
create_command!(events_json, "event json");
create_command!(events_xml, "event xml");
create_command!(events_disable, "noevents", no_args);
create_command!(disconnect, "exit", no_args);

impl<'a> Command<'a> {
    pub fn bgapi<T: Into<std::borrow::Cow<'a, str>>>(s: T, event_id: T) -> Command<'a> {
        Command {
            cmd: "bgapi ",
            args: EventBuilder!(s.into(),
                "Job-UUID" => event_id.into()
            )
            .into(),
        }
    }
    pub fn execute<T: Into<std::borrow::Cow<'a, str>>>(
        uuid: T,
        app_name: T,
        args: T,
    ) -> Command<'a> {
        let body: Cow<'a, str> = args.into();
        Command {
            cmd: "sendmsg ",
            args: EventBuilder!(uuid.into(),
                "execute-app-name" => app_name.into(),
                "call-command" => "execute";
                body
            )
            .into(),
        }
    }

    pub fn execute_async<T: Into<std::borrow::Cow<'a, str>>>(
        uuid: T,
        app_name: T,
        args: T,
        event_id: T,
    ) -> Command<'a> {
        let body: Cow<'a, str> = args.into();
        Command {
            cmd: "sendmsg ",
            args: EventBuilder!(uuid.into(),
                "Job-UUID" => event_id.into(),
                "execute-app-name" => app_name.into(),
                "call-command" => "execute",
                "async" => true,
                "event-lock" => true;
                body
            )
            .into(),
        }
    }
}
