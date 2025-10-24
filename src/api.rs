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

pub struct Inbound<T>(ESLConnection<T>);

impl<T> Inbound<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(stream: T) -> Self {
        Inbound(ESLConnection::new(stream))
    }

    pub async fn auth(&mut self, password: &str) -> Result<Reply, ESLError> {
        self.send_recv(&format!("auth {}", password)).await
    }
}

impl Inbound<TcpStream> {
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

pub struct Outbound<T> {
    conn: ESLConnection<T>,
    info: Reply,
}

impl<T> Outbound<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn handshake(stream: T, config: ESLConfig) -> Result<Outbound<T>, ConnectError> {
        let mut conn = ESLConnection::new(stream);
        let info = timeout(config.timeout, conn.send_recv("connect")).await??;
        Ok(Outbound { conn, info })
    }

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
