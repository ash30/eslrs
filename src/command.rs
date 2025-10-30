use crate::EventBuilder;
use std::{borrow::Cow, fmt::Display};

#[derive(Debug)]
pub struct Command<'a> {
    pub(crate) cmd: &'static str,
    pub(crate) args: Cow<'a, str>,
}

impl<'a> Command<'a> {
    // TODO: can we convert this to std::borrow::ToOwned?
    pub fn to_owned(&self) -> Command<'static> {
        let s = self.args.to_string();
        let res: Command<'static> = Command {
            cmd: self.cmd,
            args: s.into(),
        };
        res
    }
}

impl<'a> From<&'a str> for Command<'a> {
    fn from(value: &'a str) -> Self {
        Command {
            cmd: "",
            args: value.into(),
        }
    }
}

impl<'a> From<String> for Command<'a> {
    fn from(value: String) -> Self {
        Command {
            cmd: "",
            args: value.into(),
        }
    }
}

impl<'a, T> From<&'a T> for Command<'a>
where
    T: AsRef<str>,
{
    fn from(value: &'a T) -> Self {
        Command {
            cmd: "",
            args: value.as_ref().into(),
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
pub struct SendMessageConfig<T> {
    _async: bool,
    event_lock: bool,
    event_id: Option<T>,
    _loop: usize,
}

impl<T> Default for SendMessageConfig<T> {
    fn default() -> Self {
        Self {
            _async: false,
            event_lock: false,
            event_id: None,
            _loop: 1,
        }
    }
}
impl<T> Display for SendMessageConfig<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "async: {}", self._async)?;
        if self.event_lock {
            writeln!(f, "event-lock: {}", self.event_lock)?;
        }
        if self._loop > 1 {
            writeln!(f, "loop: {}", self._loop)?
        }
        if self.event_id.is_some() {
            writeln!(f, "Job-UUID: {}", self.event_id.as_ref().unwrap())?;
        }
        Ok(())
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
    pub fn bgapi<T, U>(s: T, event_id: U) -> Command<'a>
    where
        T: Display,
        U: Display,
    {
        Command {
            cmd: "bgapi ",
            args: format!("{}\nJob-UUID: {}\n", s, event_id).into(),
        }
    }
    pub fn execute<T1, T2, T3>(uuid: T1, app_name: T2, args: T3) -> Command<'a>
    where
        T1: Into<std::borrow::Cow<'a, str>>,
        T2: Into<std::borrow::Cow<'a, str>>,
        T3: Into<std::borrow::Cow<'a, str>>,
    {
        let config: SendMessageConfig<String> = Default::default();
        Command::execute_with_config(uuid, app_name, args, config)
    }

    pub fn execute_aync<T1, T2, T3, T4>(
        uuid: T1,
        app_name: T2,
        args: T3,
        event_id: T4,
    ) -> Command<'a>
    where
        T1: Into<std::borrow::Cow<'a, str>>,
        T2: Into<std::borrow::Cow<'a, str>>,
        T3: Into<std::borrow::Cow<'a, str>>,
        T4: Display,
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

    pub fn execute_with_config<T1, T2, T3, T4>(
        uuid: T1,
        app_name: T2,
        args: T3,
        config: SendMessageConfig<T4>,
    ) -> Command<'a>
    where
        T1: Into<std::borrow::Cow<'a, str>>,
        T2: Into<std::borrow::Cow<'a, str>>,
        T3: Into<std::borrow::Cow<'a, str>>,
        T4: Display,
    {
        let body = args.into();
        let uuid: Cow<'a, str> = uuid.into();
        let app_name: Cow<'a, str> = app_name.into();

        let event = EventBuilder!(uuid,
            "execute-app-name" => app_name,
            "call-command" => "execute",
            => config;
            body
        );

        Command {
            cmd: "sendmsg ",
            args: event.into(),
        }
    }
}
