// Usage:
//   # Run with info-level tracing
//   RUST_LOG=info cargo run --example inbound

use eslrs::{Command, ESLError, EventBuilder, event::EventArg, event::EventExt};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<(), ESLError> {
    // Optional: Setup tracing subscriber or logger of choice
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let mut conn = eslrs::Inbound::connect("0.0.0.0:8021", "ClueCon")
        .await
        .unwrap();
    tracing::info!("Successfully connected to FreeSWITCH");

    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                event = conn.recv() => {
                    let Ok(event) = event else { break };
                    if event.is_json() && let Some(Ok(json)) = event.cast().json() {
                        tracing::info!("JSON response: {}", serde_json::to_string_pretty(&json).unwrap_or_default());
                    };
                }
                cmd = rx.recv() => {
                    let Some(cmd) = cmd else { break };
                    let _ = conn.send_recv(cmd).await;
                }
            }
        }
        if !conn.is_disconnected() {
            conn.disconnect().await;
        }
    });

    // Send Commands via `Command` methods
    tx.send(Command::api("status")).await.ok();
    tx.send(Command::bgapi("version", "some_uuid")).await.ok();

    // OR construct them yourself for full flexibility
    tx.send("api uptime".into()).await.ok();

    // Enable events
    tx.send(Command::events_json("all")).await.ok();

    // EventBuilder will set content-type and length
    // headers for any suitable EventFormat
    let e = EventBuilder!(
        "SEND_MESSAGE",
        "profile" => "internal",
        "user" => 100; // last header ends with ';' !!!

        "plain_text"
    );
    let _ = tx.send(Command::sendevent(e)).await;

    let _ = tx.send(Command::execute("uuid", "originate", "")).await;
    //tx.send(Command::events_disable()).await;

    // shutdown
    drop(tx);
    handle.await.ok();

    tracing::info!("Example completed");

    Ok(())
}
