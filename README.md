eslrs is a pure Rust implementation of libesl, [FreeSWITCH™](https://freeswitch.com/)'s Event Socket Library.

# Features 
- **Async**: Built on tokio for high-performance async I/O
- **Inbound and Outbound**: Support for both ESL connection modes
- **Multiple Event Formats**: JSON, plain text, and XML event parsing
- **Instrumentation**: Optional tracing/logging integration

# Quick Start

Add eslrs as dependency and enable the json feature if planning to use json events. 
```toml
eslrs = { version = "0.1", features = ["json"] }
```

# Inbound and Outbound APIs 
 ## Inbound 
 ```
 use eslrs::{Inbound, Command, event::EventExt};

 #[tokio::main]
 async fn main() -> Result<(), Box<dyn std::error::Error>> {
     // Connect and authenticate
     let mut conn = Inbound::connect("0.0.0.0:8021", "ClueCon").await?;

     // Send commands
     let reply = conn.send_recv(Command::api("status")).await?;
     println!("Status: {:?}", reply);

     // Subscribe to events
     conn.send_recv(Command::events_json("all")).await?;

     // Receive events
     loop {
         let event = conn.recv().await?;
         if event.is_json() {
             let json = event.cast().json();
             println!("Event: {:?}", json);
         }
     }
 }
 ```

 ### Outbound 

 ```
 use eslrs::{Outbound, ESLConfig, Command};
 use tokio::net::TcpListener;

 #[tokio::main]
 async fn main() -> Result<(), Box<dyn std::error::Error>> {
     let listener = TcpListener::bind("0.0.0.0:8888").await?;
     println!("Listening for FreeSWITCH connections...");

     loop {
         let (socket, _) = listener.accept().await?;
         tokio::spawn(async move {
             let config = ESLConfig::default();
             let mut conn = Outbound::handshake(socket, config).await.unwrap();

             // Get call info
             let info = conn.get_info();
             let uuid = info.get_header("Unique-ID").unwrap();

             // Control the call
             conn.send_recv(Command::execute(uuid, "answer", "")).await.unwrap();
             conn.send_recv(Command::execute(uuid, "playback", "/tmp/hello.wav")).await.unwrap();
         });
     }
 }
 ```

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
