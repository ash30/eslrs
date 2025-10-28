use eslrs::{Command, ConnectError, ESLConfig, ESLError, EventBuilder, event::EventExt};
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
            let mut conn = eslrs::Outbound::handshake(socket, config.to_owned()).await?;
            // Get call info
            let info = conn.get_info().clone();
            let uuid = info.get_header("Unique-ID").unwrap();

            // Control the call
            conn.send_recv(Command::execute(uuid, "answer", ""))
                .await
                .unwrap();
            conn.send_recv(Command::execute(uuid, "playback", "/tmp/hello.wav"))
                .await
                .unwrap();

            Ok::<(), ConnectError>(())
        });
    }
}
