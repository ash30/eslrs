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
    ($(#[$meta:meta])* $name:ident) => {
        create_command!($(#[$meta])* $name, stringify!($name));
    };
    ($(#[$meta:meta])* $name:ident, $cmd:expr) => {
        impl<'a> Command<'a> {
            $(#[$meta])*
            pub fn $name<T: Into<std::borrow::Cow<'a, str>>>(s: T) -> Command<'a> {
                Command {
                    cmd: concat!($cmd, " "),
                    args: s.into(),
                }
            }
        }
    };
    ($(#[$meta:meta])* $name:ident, $cmd:expr, no_args) => {
        impl<'a> Command<'a> {
            $(#[$meta])*
            pub fn $name() -> Command<'a> {
                Command {
                    cmd: concat!($cmd, " "),
                    args: "".into(),
                }
            }
        }
    };
}

create_command!(
    /// Executes an API command synchronously.
    ///
    /// # Arguments
    ///
    /// * `cmd` - freeswitch api command and args
    ///
    /// # Examples
    ///
    /// ```
    /// use eslrs::Command;
    /// Command::api("status");
    /// Command::api("version");
    /// Command::api("global_getvar domain");
    /// ```
    api
);

create_command!(
    /// Only events matching the filter will be received.
    /// Additive if called multiple times.
    ///
    /// # Arguments
    ///
    /// * `filter` - \<EventHeader\> \<ValueToFilter\>
    ///
    /// # Examples
    ///
    /// ```
    /// use eslrs::Command;
    /// // Only receive events for a specific channel UUID
    /// Command::filter("Unique-ID abc123");
    ///
    /// // Filter by event name
    /// Command::filter("Event-Name CHANNEL_HANGUP");
    /// ```
    filter);

create_command!(
    /// Removes a previously set event filter.
    ///
    /// # Examples
    ///
    /// ```
    /// use eslrs::Command;
    /// Command::filter_delete("Unique-ID abc123");
    /// ```
    filter_delete, "filter delete");

create_command!(
    /// Subscribes to events in Plain format.
    ///
    /// # Arguments
    ///
    /// * `events` - Space-separated event names or "all" for all events
    ///
    /// # Examples
    ///
    /// ```
    /// use eslrs::Command;
    ///
    /// // Subscribe to events in json format
    /// Command::events("all");
    ///
    /// // Subscribe to specific events
    /// Command::events("CHANNEL_CREATE CHANNEL_DESTROY");
    /// ```

    events, "event plain");

create_command!(
    /// Sends a custom event to FreeSWITCH.
    ///
    /// Use with the `EventBuilder!` macro to construct event data.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eslrs::{Command, EventBuilder, Inbound};
    /// let event = EventBuilder!(
    ///     "CUSTOM",
    ///     "Event-Subclass" => "myapp::event",
    ///     "Custom-Header" => "value";
    ///
    ///     "Event body content"
    /// );
    ///
    /// Command::sendevent(event);
    /// ```
sendevent);

create_command!(
    /// Subscribes to events in JSON format.
    ///
    /// # Arguments
    ///
    /// * `events` - Space-separated event names or "all" for all events
    ///
    /// # Examples
    ///
    /// ```
    /// use eslrs::Command;
    ///
    /// // Subscribe to events in json format
    /// Command::events_json("all");
    ///
    /// // Subscribe to specific events
    /// Command::events_json("CHANNEL_CREATE CHANNEL_DESTROY");
    /// ```
    events_json, "event json");

// TODO: impl!
//create_command!(events_xml, "event xml");

create_command!(
    /// Disables all event subscriptions.
    events_disable, "noevents", no_args);

create_command!(
    /// Disconnects from FreeSWITCH.
    disconnect, "exit", no_args);

#[derive(Debug, Clone)]
pub struct SendMessageConfig<'a> {
    _async: bool,
    event_lock: bool,
    event_id: Option<&'a str>,
    _loop: usize,
}

impl<'a> Default for SendMessageConfig<'a> {
    fn default() -> Self {
        Self {
            _async: false,
            event_lock: false,
            event_id: None,
            _loop: 1,
        }
    }
}

impl<'a> Command<'a> {
    /// Executes an API command asynchronously in the background.
    ///
    /// Returns immediately with a Job-UUID. The result is delivered
    /// as a BACKGROUND_JOB event.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use eslrs::{Command, Inbound, event::EventExt};
    ///
    /// let job_uuid = "unique-job-id";
    /// Command::bgapi("status", job_uuid);
    ///
    /// // Later, receive the BACKGROUND_JOB event
    /// if let Some(uuid) = event.get_header("Job-UUID") {
    ///     if uuid == job_uuid {
    ///         // This is our result
    ///     }
    /// }
    /// ```
    pub fn bgapi<T: Into<std::borrow::Cow<'a, str>>>(s: T, event_id: &'a str) -> Command<'a> {
        Command {
            cmd: "bgapi ",
            args: format!("{}\nJob-UUID: {}\n", s.into(), event_id).into(),
        }
    }
    pub fn execute<T1, T2, T3>(uuid: T1, app_name: T2, args: T3) -> Command<'a>
    where
        T1: Into<std::borrow::Cow<'a, str>>,
        T2: Into<std::borrow::Cow<'a, str>>,
        T3: Into<std::borrow::Cow<'a, str>>,
    {
        Command::execute_with_config(uuid, app_name, args, Default::default())
    }

    pub fn execute_aync<T1, T2, T3>(
        uuid: T1,
        app_name: T2,
        args: T3,
        event_id: &'a str,
    ) -> Command<'a>
    where
        T1: Into<std::borrow::Cow<'a, str>>,
        T2: Into<std::borrow::Cow<'a, str>>,
        T3: Into<std::borrow::Cow<'a, str>>,
    {
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

    pub fn execute_with_config<T1, T2, T3>(
        uuid: T1,
        app_name: T2,
        args: T3,
        config: SendMessageConfig<'a>,
    ) -> Command<'a>
    where
        T1: Into<std::borrow::Cow<'a, str>>,
        T2: Into<std::borrow::Cow<'a, str>>,
        T3: Into<std::borrow::Cow<'a, str>>,
    {
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
