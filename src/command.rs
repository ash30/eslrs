use crate::{EventBuilder, event::Header};
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
            args: format!("{}\nJob-UUID: {}\n", s.into(), event_id).into(),
        }
    }
    pub fn execute<T: Into<std::borrow::Cow<'a, str>>>(
        uuid: T,
        app_name: T,
        args: T,
    ) -> Command<'a> {
        Command::execute_with_config(uuid, app_name, args, Default::default())
    }

    pub fn execute_aync<T: Into<std::borrow::Cow<'a, str>>>(
        uuid: T,
        app_name: T,
        args: T,
        event_id: &'a str,
    ) -> Command<'a> {
        Command::execute_with_config(
            uuid,
            app_name,
            args,
            SendMessageConfig {
                _async: true,
                event_lock: true,
                event_id: Some(event_id),
                ..Default::default()
            },
        )
    }

    pub fn execute_with_config<T: Into<std::borrow::Cow<'a, str>>>(
        uuid: T,
        app_name: T,
        args: T,
        config: SendMessageConfig<'a>,
    ) -> Command<'a> {
        let body = args.into();
        let uuid: Cow<'a, str> = uuid.into();
        let app_name: Cow<'a, str> = app_name.into();

        let _async = Header!("async" => config._async);
        let l = if config.event_lock {
            Header!("event-lock" => 1)
        } else {
            format_args!("")
        };
        let id = config.event_id.unwrap_or("");
        let event_id = if !id.is_empty() {
            Header!("Job-UUID" => id)
        } else {
            format_args!("")
        };

        let _loop = Header!("loop" => config._loop);
        let additional = format_args!("{}{}{}{}", _async, l, event_id, _loop);

        let event = EventBuilder!(uuid,
            "execute-app-name" => app_name,
            "call-command" => "execute",
            => additional;
            body
        );

        Command {
            cmd: "sendmsg ",
            args: event.into(),
        }
    }
}
