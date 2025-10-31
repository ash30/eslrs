use std::{
    error::Error,
    fmt::{self, Display},
    ops::{Deref, DerefMut},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::{TcpStream, ToSocketAddrs},
    time::{error::Elapsed, timeout},
};

use crate::{ESLConnection, ESLError, event::Reply};

#[derive(Debug, Clone)]
pub struct ESLConfig {
    pub password: String,
    pub timeout: Duration,
}

impl Default for ESLConfig {
    fn default() -> Self {
        Self {
            password: "".to_string(),
            timeout: Duration::from_secs(5),
        }
    }
}

impl From<&str> for ESLConfig {
    fn from(value: &str) -> Self {
        Self {
            password: value.to_string(),
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub enum ConnectError {
    Timeout,
    Auth,
    Connection(ESLError),
}

impl From<Elapsed> for ConnectError {
    fn from(_: Elapsed) -> Self {
        ConnectError::Timeout
    }
}
impl From<ESLError> for ConnectError {
    fn from(value: ESLError) -> Self {
        ConnectError::Connection(value)
    }
}
impl From<std::io::Error> for ConnectError {
    fn from(value: std::io::Error) -> Self {
        Self::Connection(ESLError::from(value))
    }
}
impl Error for ConnectError {}
impl Display for ConnectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct Inbound<T = TcpStream>(ESLConnection<T>);

impl Inbound<TcpStream> {
    /// Connects to a FreeSWITCH Event Socket and authenticates.
    ///
    /// This is the primary entry point for inbound ESL connections. It will:
    /// 1. Establish a TCP connection to the FreeSWITCH event socket
    /// 2. Automatically authenticate with the provided password
    /// 3. Return an authenticated connection ready for use
    ///
    /// # Arguments
    ///
    /// * `addr` - The socket address (e.g., "127.0.0.1:8021" or "0.0.0.0:8021")
    /// * `config` - Either an `ESLConfig` struct or a password string (via `Into`)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eslrs::Inbound;
    /// use eslrs::ESLConfig;
    /// use std::time::Duration;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// // Connect with just a password (uses default timeout of 5 seconds)
    /// let mut conn = Inbound::connect("0.0.0.0:8021", "ClueCon").await.unwrap();
    ///
    /// // Or with full config
    /// let config = ESLConfig {
    ///     password: "ClueCon".to_string(),
    ///     timeout: Duration::from_secs(10),
    /// };
    /// let mut conn = Inbound::connect("0.0.0.0:8021", config).await.unwrap();
    /// # }
    /// ```
    pub async fn connect<U: ToSocketAddrs, V: Into<ESLConfig>>(
        addr: U,
        config: V,
    ) -> Result<Inbound<TcpStream>, ConnectError> {
        let config: ESLConfig = config.into();
        let stream = timeout(config.timeout, TcpStream::connect(addr)).await??;
        let mut conn = Inbound::new(stream);
        if conn.auth(&config.password).await.is_ok() {
            Ok(conn)
        } else {
            Err(ConnectError::Auth)
        }
    }
}

impl<T> Inbound<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    /// Creates a new inbound connection from an existing stream.
    ///
    /// Use this when you have a custom stream type.
    /// For standard TCP connections, prefer [`Inbound::connect`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eslrs::Inbound;
    /// use tokio::net::TcpStream;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let stream = TcpStream::connect("0.0.0.0:8021").await.unwrap();
    /// let mut conn = Inbound::new(stream);
    /// # }
    /// ```
    pub fn new(stream: T) -> Self {
        Inbound(ESLConnection::new(stream))
    }

    /// Authenticates with FreeSWITCH using the provided password.
    ///
    /// Called automatically by [`Inbound::connect`]. Only needed when using
    /// [`Inbound::new`] with a custom stream.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eslrs::{Inbound, ESLError};
    /// use tokio::net::TcpStream;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let stream = TcpStream::connect("0.0.0.0:8021").await.unwrap();
    /// let mut conn = Inbound::new(stream);
    /// let reply = conn.auth("ClueCon").await.unwrap();
    /// assert!(reply.is_ok());
    /// # }
    /// ```
    pub async fn auth(&mut self, password: &str) -> Result<Reply, ESLError> {
        self.send_recv(&format!("auth {}", password)).await
    }
}

impl<T> Deref for Inbound<T> {
    type Target = ESLConnection<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for Inbound<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// =============

pub struct Outbound<T = TcpStream> {
    conn: ESLConnection<T>,
    info: Reply,
}

impl<T> Outbound<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    /// Performs the outbound socket handshake with FreeSWITCH.
    ///
    /// In outbound mode, FreeSWITCH initiates the connection to your application.
    /// This method performs the handshake by sending the "connect" command and
    /// receiving channel/call information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eslrs::{Outbound, ESLConfig};
    /// use tokio::net::TcpListener;
    ///
    /// # #[tokio::main(flavor = "current_thread")]
    /// # async fn main() {
    /// let listener = TcpListener::bind("0.0.0.0:8888").await.unwrap();
    ///
    /// loop {
    ///     let (socket, _) = listener.accept().await.unwrap();
    ///     tokio::spawn(async move {
    ///         let config = ESLConfig::default();
    ///         let mut conn = Outbound::handshake(socket, config).await.unwrap();
    ///
    ///         // Access call info from the handshake
    ///         let info = conn.get_info();
    ///         let caller_id = info.get_header("Caller-Caller-ID-Number");
    ///
    ///         // Control the call...
    ///     });
    /// }
    /// # }  
    /// ```
    pub async fn handshake(stream: T, config: ESLConfig) -> Result<Outbound<T>, ConnectError> {
        let mut conn = ESLConnection::new(stream);
        let info = timeout(config.timeout, conn.send_recv("connect")).await??;
        Ok(Outbound { conn, info })
    }

    /// Returns the channel information received during the handshake.
    ///
    /// This contains all the channel variables available at the time
    /// FreeSWITCH connected to your outbound socket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eslrs::{Outbound, ESLConfig};
    /// # async fn example(conn: Outbound<tokio::net::TcpStream>) {
    /// let info = conn.get_info();
    /// if let Some(uuid) = info.get_header("Unique-ID") {
    ///     println!("Channel UUID: {}", uuid);
    /// }
    /// if let Some(dest) = info.get_header("Caller-Destination-Number") {
    ///     println!("Dialed number: {}", dest);
    /// }
    /// # }
    /// ```
    pub fn get_info(&self) -> &Reply {
        &self.info
    }
}

impl<T> Deref for Outbound<T> {
    type Target = ESLConnection<T>;
    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}
impl<T> DerefMut for Outbound<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}
