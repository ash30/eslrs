eslrs is a pure Rust implementation of libesl, [FreeSWITCH™](https://freeswitch.com/)'s Event Socket Library.

# Features 
- Async 
- Inbound and Outbound APIs

# Quick Start

Add eslrs as dependency and enable the json feature if planning to use json events. 
```toml
eslrs = { version = "0.1", features = ["json"] }
```

# Inbound and Outbound APIs 

BLAH
BLAH
BLAH

# Event Formats 

```
let res = conn.send_recv(Command::events_json("all")).await?;
let event = conn.recv()?;
if event.is_json() {
    dbg!(e.cast().json());
}
else {
    dbg!(e.get_content_type(), e.bytes());
}
```

# Logging and Tracing

The main api of the esl connection have been instrumented to feed into either logging and tracing eco-systems.

The tracing feature is enabled by default, logging requires manual inclusion.
See tracing_example.rs within the repo for more details.

```logs
2025-10-23T14:57:21.225778Z  INFO tracing_example: Connecting to FreeSWITCH at 0.0.0.0:8021
2025-10-23T14:57:21.839107Z  INFO send_recv{command="auth ClueCon"}: eslrs::connection: return=Reply(RawEvent { headers: {"Content-Type": "command/reply", "Reply-Text": "+OK accepted"}, body: None })
2025-10-23T14:57:21.839399Z  INFO recv: eslrs::connection: return=Event(headers={"Content-Type": "auth/request"})
```
