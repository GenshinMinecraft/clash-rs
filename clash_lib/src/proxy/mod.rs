use crate::config::internal::proxy::OutboundProxy;
use crate::proxy::datagram::UdpPacket;
use crate::proxy::utils::Interface;
use crate::session::{Session, SocksAddr};
use crate::ThreadSafeDNSResolver;
use async_trait::async_trait;
use futures::{Sink, Stream};
use std::fmt::Debug;
use std::io;
use std::sync::Arc;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;

pub mod direct;
pub mod reject;

pub(crate) mod datagram;
pub mod http;
#[cfg(feature = "shadowsocks")]
pub mod shadowsocks;
pub mod socks;
//pub mod trojan;
pub mod utils;
//pub mod vmess;

pub mod converters;

// proxy groups
pub mod relay;
pub mod selector;

mod transport;

#[cfg(test)]
use mockall::automock;

#[derive(thiserror::Error, Debug)]
pub enum ProxyError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("proxy error: {0}")]
    General(String),
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("socks5 error: {0}")]
    Socks5(String),
}

pub trait ProxyStream: AsyncRead + AsyncWrite + Send + Sync + Unpin + Debug {}
impl<T> ProxyStream for T where T: AsyncRead + AsyncWrite + Send + Sync + Unpin + Debug {}
pub type AnyStream = Box<dyn ProxyStream>;

pub trait InboundDatagram<Item>:
    Stream<Item = Item> + Sink<Item, Error = io::Error> + Send + Sync + Unpin + Debug
{
}
pub type AnyInboundDatagram =
    Box<dyn InboundDatagram<UdpPacket, Error = io::Error, Item = UdpPacket>>;

pub trait OutboundDatagram<Item>:
    Stream<Item = Item> + Sink<Item, Error = io::Error> + Send + Sync + Unpin
{
}

impl<T, U> OutboundDatagram<U> for T where
    T: Stream<Item = U> + Sink<U, Error = io::Error> + Send + Sync + Unpin
{
}

pub type AnyOutboundDatagram =
    Box<dyn OutboundDatagram<UdpPacket, Item = UdpPacket, Error = io::Error>>;

#[async_trait]
pub trait OutboundDatagramRecvHalf: Sync + Send + Unpin {
    /// Receives a message on the socket. On success, returns the number of
    /// bytes read and the origin of the message.
    async fn recv_from(&mut self, buf: &mut [u8]) -> io::Result<(usize, SocksAddr)>;
}

/// The send half.
#[async_trait]
pub trait OutboundDatagramSendHalf: Sync + Send + Unpin {
    /// Sends a message on the socket to `dst_addr`. On success, returns the
    /// number of bytes sent.
    async fn send_to(&mut self, buf: &[u8], dst_addr: &SocksAddr) -> io::Result<usize>;
}

#[derive(Default)]
pub struct CommonOption {
    so_mark: Option<u32>,
    iface: Option<Interface>,
}

#[async_trait]
pub trait InboundListener: Send + Sync + Unpin {
    /// support tcp or not
    fn handle_tcp(&self) -> bool;
    /// support udp or not
    fn handle_udp(&self) -> bool;
    async fn listen_tcp(&self) -> io::Result<()>;
    async fn listen_udp(&self) -> io::Result<()>;
}

pub type AnyInboundListener = Arc<dyn InboundListener>;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait OutboundHandler: Sync + Send + Unpin {
    /// The name of the outbound handler
    fn name(&self) -> &str;

    /// The protocol of the outbound handler
    /// only contains Type information, do not rely on the underlying value
    fn proto(&self) -> OutboundProxy;

    /// The proxy remote address
    async fn remote_addr(&self) -> Option<SocksAddr>;

    /// whether the outbound handler support UDP
    async fn support_udp(&self) -> bool;

    /// connect to remote target via TCP
    async fn connect_stream(
        &self,
        sess: &Session,
        resolver: ThreadSafeDNSResolver,
    ) -> io::Result<AnyStream>;

    /// wraps a stream with outbound handler
    async fn proxy_stream(
        &self,
        s: AnyStream,
        sess: &Session,
        resolver: ThreadSafeDNSResolver,
    ) -> io::Result<AnyStream>;

    /// connect to remote target via UDP
    async fn connect_datagram(
        &self,
        sess: &Session,
        resolver: ThreadSafeDNSResolver,
    ) -> io::Result<AnyOutboundDatagram>;
}
pub type AnyOutboundHandler = Arc<dyn OutboundHandler>;
