mod config;
mod network_config;
#[cfg(feature = "quic")]
pub mod quic;
pub mod tcp;
pub mod udp;

pub use crate::node::blocking::config::*;
pub use crate::node::blocking::network_config::NetworkConfig;
#[cfg(feature = "quic")]
pub use crate::node::blocking::network_config::Quic;
pub use crate::node::blocking::network_config::{Tcp, Udp};
#[cfg(feature = "quic")]
pub use crate::node::blocking::quic::*;
pub use crate::node::blocking::tcp::*;

extern crate alloc;

use tokio::net::{TcpStream, UdpSocket};
use tokio::runtime::{Handle, Runtime};
use tokio::sync::Mutex as TokioMutex;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

use tracing::*;

use std::net::SocketAddr;

use std::marker::{PhantomData, Sync};
use std::result::Result;
use std::sync::Arc;

use alloc::vec::Vec;
use postcard::*;

use crate::msg::*;
use crate::node::blocking::network_config::Interface;
use crate::Error;
use chrono::{DateTime, Utc};

// Quic stuff
#[cfg(feature = "quic")]
use quinn::Connection as QuicConnection;
#[cfg(feature = "quic")]
use quinn::{ClientConfig, Endpoint};
#[cfg(feature = "quic")]
use rustls::Certificate;
#[cfg(feature = "quic")]
use std::fs::File;
#[cfg(feature = "quic")]
use std::io::BufReader;

/// State marker for a Node that has not been connected to a Host
#[derive(Debug)]
pub struct Idle;
/// State marker for a Node capable of manually sending publish/request messages
#[derive(Debug)]
pub struct Active;
/// State marker for a Node with an active topic subscription
#[derive(Debug)]
pub struct Subscription;

mod private {
    pub trait Sealed {}
    impl Sealed for crate::Udp {}
    impl Sealed for crate::Tcp {}
    #[cfg(feature = "quic")]
    impl Sealed for crate::node::blocking::network_config::Quic {}

    impl Sealed for crate::Idle {}
    impl Sealed for crate::Active {}
}

use std::sync::Mutex;

/// Strongly-typed Node capable of publish/request on Host
#[derive(Debug)]
pub struct Node<I: Interface + Default, State, T: Message> {
    pub __state: PhantomData<State>,
    pub __data_type: PhantomData<T>,
    pub cfg: NodeConfig<I, T>,
    pub runtime: Option<Runtime>,
    pub rt_handle: Handle,
    pub topic: String,
    pub stream: Option<TcpStream>,
    pub socket: Option<UdpSocket>,
    pub buffer: Arc<TokioMutex<Vec<u8>>>,
    #[cfg(feature = "quic")]
    pub endpoint: Option<Endpoint>,
    #[cfg(feature = "quic")]
    pub connection: Option<QuicConnection>,
    pub subscription_data: Arc<TokioMutex<Option<Msg<T>>>>,
    pub task_subscribe: Option<JoinHandle<()>>,
}
