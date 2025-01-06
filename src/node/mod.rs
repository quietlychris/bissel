pub mod config;
pub mod network_config;
pub mod tcp;
pub mod udp;

#[cfg(feature = "quic")]
pub mod quic;

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

    use crate::node::network_config::{Tcp, Udp};
    impl Sealed for Udp {}
    impl Sealed for Tcp {}
    #[cfg(feature = "quic")]
    impl Sealed for crate::node::network_config::Quic {}

    use crate::node::{Active, Idle};
    impl Sealed for Idle {}
    impl Sealed for Active {}

    use crate::node::network_config::{Blocking, Nonblocking};
    impl Sealed for Blocking {}
    impl Sealed for Nonblocking {}
}

use tokio::io::AsyncWriteExt;
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

extern crate alloc;
use alloc::vec::Vec;
use postcard::*;

use crate::msg::*;
use crate::node::network_config::{Block, Interface};
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

use crate::node::config::NodeConfig;
use std::sync::Mutex;

/// Strongly-typed Node capable of publish/request on Host
#[derive(Debug)]
pub struct Node<B: Block, I: Interface + Default, State, T: Message> {
    pub __state: PhantomData<State>,
    pub __data_type: PhantomData<T>,
    pub cfg: NodeConfig<B, I, T>,
    pub runtime: Option<Runtime>,
    pub rt_handle: Option<Handle>,
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
