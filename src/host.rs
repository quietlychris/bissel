// Tokio for async
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio::sync::Mutex; // as TokioMutex;

use postcard::*;

use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::ops::DerefMut;

use std::error::Error;
use std::net::SocketAddr;
use std::result::Result;

use crate::msg::*;

#[derive(Debug)]
pub struct Host {
    cfg: HostConfig,
    connections: Arc<StdMutex<Vec<JoinHandle<()>>>>,
    task_listen: Option<JoinHandle<()>>,
    store: sled::Db,
    reply_count: Arc<Mutex<usize>>,
}

#[derive(Debug)]
pub struct HostConfig {
    interface: String,
    socket_num: usize,
    store_name: String,
}

impl HostConfig {
    pub fn new(interface: impl Into<String>) -> HostConfig {
        HostConfig {
            interface: interface.into(),
            socket_num: 25_000,
            store_name: "rhiza_store".into(),
        }
    }

    pub fn socket_num(mut self, socket_num: usize) -> HostConfig {
        self.socket_num = socket_num;
        self
    }

    pub fn store_name(mut self, store_name: impl Into<String>) -> HostConfig {
        self.store_name = store_name.into();
        self
    }
}

impl Host {
    pub fn from_config(cfg: HostConfig) -> Result<Host, Box<dyn Error>> {
        let ip = crate::get_ip(&cfg.interface)?;
        println!(
            "On interface {:?}, the device IP is: {:?}",
            &cfg.interface, &ip
        );

        let raw_addr = ip.to_owned() + ":" + &cfg.socket_num.to_string();
        // If the address won't parse, this should panic
        let _addr: SocketAddr = raw_addr.parse()
            .expect(&format!("The provided address string, \"{}\" is invalid",&raw_addr));
        let connections = Arc::new(StdMutex::new(Vec::new()));

        let config = sled::Config::default().path(&cfg.store_name).temporary(true);
        let store: sled::Db = config.open()?;
   
        let reply_count = Arc::new(Mutex::new(0));

        Ok(Host {
            cfg,
            connections,
            task_listen: None,
            store,
            reply_count
        })
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn Error>> {
        
        let ip = crate::get_ip(&self.cfg.interface)?;
        let raw_addr = ip.to_owned() + ":" + &self.cfg.socket_num.to_string();
        let addr: SocketAddr = raw_addr.parse()?;
        let listener = TcpListener::bind(addr).await?;
        let connections_clone = self.connections.clone();

        let db = self.store.clone();
        

        let counter = self.reply_count.clone();

        let task_listen = tokio::spawn( async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let db = db.clone();
                let counter = counter.clone();
                let connections = Arc::clone(&connections_clone.clone());
                let handle = tokio::spawn(async move {
                    process(stream, db, counter).await;
                });
                connections.lock().unwrap().push(handle);
            }
        });

        self.task_listen = Some(task_listen);




        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn Error>> {
                
        for handle in self.connections.lock().unwrap().deref_mut() {
            handle.abort();
        }
        Ok(())
    }
}

async fn process(stream: TcpStream, db: sled::Db, count: Arc<Mutex<usize>>) {
    let mut buf = [0u8; 4096];
    loop {
        stream.readable().await.unwrap();
        dbg!(&count);
        match stream.try_read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                stream.writable().await.unwrap();

                let bytes = &buf[..n];
                let msg: GenericRhizaMsg = from_bytes(bytes).unwrap();
                dbg!(&msg);

                match msg.msg_type {
                    Msg::SET => {
                        println!("received {} bytes, to be assigned to: {}", n, &msg.name);
                        db.insert(msg.name.as_bytes(), bytes).unwrap();
                    }
                    Msg::GET => loop {
                        println!(
                            "received {} bytes, asking for reply on topic: {}",
                            n, &msg.name
                        );

                        println!("Wait for stream to be writeable");
                        // stream.writable().await.unwrap();
                        println!("Stream now writeable");
                        let return_bytes = db.get(&msg.name).unwrap().unwrap();
                        match stream.try_write(&return_bytes) {
                            Ok(n) => {
                                println!("Successfully replied with {} bytes", n);
                                let mut count = count.lock().await; //.unwrap();
                                *count += 1;
                                break;
                            }
                            Err(e) => {
                                if e.kind() == std::io::ErrorKind::WouldBlock {}
                                continue;
                            }
                        }
                    },
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // println!("Error::WouldBlock: {:?}", e);
                continue;
            }
            Err(e) => {
                println!("Error: {:?}", e);
                // return Err(e.into());
            }
        }
    }
}
