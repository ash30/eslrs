use eslrs::{Command, ESLConfig, ESLError, EventBuilder, event::EventExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), ESLError> {
    let addr = "0.0.0.0:8888"; // Listening address
    println!("Listening on {}", addr);
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (socket, _) = listener.accept().await?;
        let config = ESLConfig::default();
        tokio::spawn(async move {
            let conn = eslrs::Outbound::handshake(socket, config.to_owned()).await;
        });
    }
}
