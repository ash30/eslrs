// Example demonstrating logging support in eslrs using env_logger
//
// Usage:
//   # Run with info-level logging
//   RUST_LOG=info cargo run --example logging_example
//
//   # Run with debug-level logging for more details
//   RUST_LOG=debug cargo run --example logging_example
//
//   # Run with trace-level for maximum verbosity
//   RUST_LOG=trace cargo run --example logging_example
//

use esl_rs::{Command, ESLError, EventBuilder, event::EventExt};
use log::{info, warn};

#[tokio::main]
async fn main() -> Result<(), ESLError> {
    // Initialize env_logger that respects RUST_LOG environment variable
    env_logger::init();

    info!("Starting ESL logging example");
    info!("Connecting to FreeSWITCH at 0.0.0.0:8021");

    // Connect to FreeSWITCH
    let mut conn = esl_rs::connect("0.0.0.0:8021", "ClueCon").await.unwrap();
    info!("Successfully connected to FreeSWITCH");

    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    // Spawn task to handle connection events and commands
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(event) = conn.recv() => {
                    if event.is_json() {
                        if let Some(Ok(json)) = event.cast().json() {
                            info!("JSON response: {}", serde_json::to_string_pretty(&json).unwrap_or_default());
                        }
                    } else {
                        //let content = String::from_utf8_lossy(event.bytes());
                        //info!("Response: {}", content.trim());
                    }
                }
                Some(cmd) = rx.recv() => {
                    match conn.send_recv(cmd).await {
                        Ok(response) => {
                            //log::trace!("Command completed successfully");
                        }
                        Err(e) => {
                            //log::error!("Command failed: {:?}", e);
                        }
                    }
                }
                else => {
                    info!("Channel closed, ending connection handler");
                    break
                }
            }
        }
        warn!("Connection lost or closed");
    });

    // Send various API commands to demonstrate logging
    //info!("Sending 'status' command");
    tx.send(Command::api("status")).await.ok();

    //info!("Sending 'version' command");
    tx.send(Command::api("version")).await.ok();

    let _ = tx.send(Command::events("all")).await;
    let _ = tx.send(Command::events_json("all")).await;

    // EventBuilder will set content-type and length
    // headers for any suitable EventFormat
    let e = EventBuilder!(
        "SEND_MESSAGE",
        "profile" => "internal",
        "user" => 100;

        serde_json::json!({
            "content":"Ok"
        })
    );
    let _ = tx.send(Command::sendevent(e)).await;

    // Clean shutdown
    drop(tx);
    handle.await.ok();
    info!("Example completed");

    Ok(())
}
