extern crate alloc;
use crate::Error;
use crate::*;

use tokio::net::UdpSocket;
use tokio::sync::Mutex as TokioMutex;
use tokio::time::{sleep, Duration};

use tracing::*;

use std::result::Result;
use std::sync::Arc;

use alloc::vec::Vec;
use postcard::*;
use std::marker::PhantomData;

use crate::msg::*;
use chrono::Utc;

impl<T: Message> From<Node<Idle, T>> for Node<Active, T> {
    fn from(node: Node<Idle, T>) -> Self {
        Self {
            __state: PhantomData,
            phantom: PhantomData,
            cfg: node.cfg,
            runtime: node.runtime,
            stream: node.stream,
            name: node.name,
            topic: node.topic,
            socket: node.socket,
            subscription_data: node.subscription_data,
            task_subscribe: None,
        }
    }
}

impl<T: Message> From<Node<Idle, T>> for Node<Subscription, T> {
    fn from(node: Node<Idle, T>) -> Self {
        Self {
            __state: PhantomData,
            phantom: PhantomData,
            cfg: node.cfg,
            runtime: node.runtime,
            stream: node.stream,
            name: node.name,
            topic: node.topic,
            socket: node.socket,
            subscription_data: node.subscription_data,
            task_subscribe: None,
        }
    }
}

impl<T: Message + 'static> Node<Idle, T> {
    /// Attempt connection from the Node to the Host located at the specified address
    #[tracing::instrument]
    pub fn activate(mut self) -> Result<Node<Active, T>, Error> {
        let addr = self.cfg.tcp.host_addr;
        let topic = self.topic.clone();

        let stream = self.runtime.block_on(async move {
            match try_connection(addr).await {
                Ok(stream) => match handshake(stream, topic).await {
                    Ok(stream) => Ok(stream),
                    Err(e) => {
                        error!("{:?}", e);
                        Err(Error::Handshake)
                    }
                },
                Err(e) => {
                    error!("{:?}", e);
                    Err(Error::StreamConnection)
                }
            }
        });
        info!("Established Node<=>Host TCP stream: {:?}", &stream);
        match stream {
            Ok(stream) => self.stream = Some(stream),
            Err(e) => return Err(e),
        };

        match self.runtime.block_on(async move {
            // get_ip(self.cfg.udp.)
            match UdpSocket::bind("0.0.0.0:0").await {
                Ok(socket) => Ok(socket),
                Err(_e) => Err(Error::AccessSocket),
            }
        }) {
            Ok(socket) => self.socket = Some(socket),
            Err(e) => return Err(e),
        };

        Ok(Node::<Active, T>::from(self))
    }

    #[tracing::instrument]
    pub fn subscribe(mut self, rate: Duration) -> Result<Node<Subscription, T>, Error> {
        let name = self.name.clone() + "_SUBSCRIPTION";
        let addr = self.cfg.tcp.host_addr;
        let topic = self.topic.clone();

        let subscription_data: Arc<TokioMutex<Option<SubscriptionData<T>>>> =
            Arc::new(TokioMutex::new(None));
        let data = Arc::clone(&subscription_data);

        let max_buffer_size = self.cfg.tcp.max_buffer_size;
        let task_subscribe = self.runtime.spawn(async move {
            let stream = match try_connection(addr).await {
                Ok(stream) => match handshake(stream, topic.clone()).await {
                    Ok(stream) => Ok(stream),
                    Err(e) => {
                        error!("{:?}", e);
                        Err(Error::Handshake)
                    }
                },
                Err(e) => {
                    error!("{:?}", e);
                    Err(Error::StreamConnection)
                }
            }
            .unwrap();
            info!("Successfully subscribed to Host");

            let packet = GenericMsg {
                msg_type: MsgType::GET,
                timestamp: Utc::now(),
                name: name.clone(),
                topic: topic.clone(),
                data_type: std::any::type_name::<T>().to_string(),
                data: Vec::new(),
            };
            // info!("{:?}",&packet);

            loop {
                let packet_as_bytes: Vec<u8> = to_allocvec(&packet).unwrap();
                send_msg(&mut &stream, packet_as_bytes).await.unwrap();
                let reply = match await_response::<T>(&mut &stream, max_buffer_size).await {
                    Ok(val) => val,
                    Err(e) => {
                        error!("Subscription Error: {}", e);
                        continue;
                    }
                };
                let delta = Utc::now() - reply.timestamp;
                // println!("The time difference between msg tx/rx is: {} us",delta);
                if delta <= chrono::Duration::zero() {
                    // println!("Data is not newer, skipping to next subscription iteration");
                    continue;
                }
                // info!("Node has received msg data: {:?}",&msg.data);
                let reply_data = match from_bytes::<T>(&reply.data) {
                    Ok(data) => data,
                    Err(e) => {
                        error!("{:?}", e);
                        continue;
                    }
                };
                let reply_sub_data = SubscriptionData {
                    data: reply_data,
                    timestamp: reply.timestamp,
                };
                let mut data = data.lock().await;

                *data = Some(reply_sub_data);
                sleep(rate).await;
            }
        });
        self.task_subscribe = Some(task_subscribe);

        let mut subscription_node = Node::<Subscription, T>::from(self);
        subscription_node.subscription_data = subscription_data;

        Ok(subscription_node)
    }
}
