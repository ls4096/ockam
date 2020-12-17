#![allow(unused)]

extern crate alloc;

use crate::tcp_worker::TcpWorker;
use alloc::rc::Rc;
use libc_print::*;
use ockam::message::MAX_MESSAGE_SIZE;
use ockam::message::{Address, Message};
use ockam_no_std_traits::{EnqueueMessage, Poll, ProcessMessage, TransportListenCallback};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::ops::Deref;
use std::str::FromStr;
use std::time::Duration;

pub struct TcpRouter {
    connections: HashMap<String, TcpWorker>,
    listener: Option<TcpListener>,
}

impl TcpRouter {
    pub fn new(
        listen_addr: Option<&str>,
        connect_callback: Option<Rc<RefCell<dyn TransportListenCallback>>>,
    ) -> Result<Self, String> {
        let connections: HashMap<String, TcpWorker> = HashMap::new();
        return match listen_addr {
            Some(la) => {
                if let Ok(l) = TcpListener::bind(la) {
                    l.set_nonblocking(true).unwrap();
                    Ok(TcpRouter {
                        connections,
                        listener: Some(l),
                    })
                } else {
                    Err("failed to bind tcp listener".into())
                }
            }
            None => Ok(TcpRouter {
                connections,
                listener: None,
            }),
        };
    }

    fn accept_new_connections(&mut self) -> Result<bool, String> {
        let mut keep_going = true;
        if let Some(listener) = &self.listener {
            for s in listener.incoming() {
                match s {
                    Ok(stream) => {
                        stream.set_nonblocking(true).unwrap();
                        let peer_addr = stream.peer_addr().unwrap().clone();
                        let tcp_worker = TcpWorker::new_connection(stream);
                        self.connections.insert(peer_addr.to_string(), tcp_worker);
                    }
                    Err(e) => match e.kind() {
                        io::ErrorKind::WouldBlock => {
                            break;
                        }
                        _ => {
                            println!("tcp listen error");
                            return Ok(false);
                        }
                    },
                }
            }
        }
        Ok(true)
    }

    pub fn try_connect(&mut self, address: &str, timeout: Option<u64>) -> Result<Address, String> {
        let sock_addr = SocketAddr::from_str(address);
        if let Err(e) = sock_addr {
            return Err("bad socket address".into());
        }
        let sock_addr = sock_addr.unwrap();
        let stream = match timeout {
            Some(to) => TcpStream::connect_timeout(&sock_addr, Duration::from_millis(to)),
            None => TcpStream::connect(&sock_addr),
        };
        match stream {
            Ok(stream) => {
                stream.set_nonblocking(true).unwrap();
                let peer_addr = stream.peer_addr().unwrap().clone();
                let tcp_worker = TcpWorker::new_connection(stream);
                self.connections.insert(peer_addr.to_string(), tcp_worker);
                Ok((Address::TcpAddress(SocketAddr::from_str(address).unwrap())))
            }
            Err(e) => Err(format!("tcp failed to connect: {}", e)),
        }
    }
}

impl ProcessMessage for TcpRouter {
    fn process_message(
        &mut self,
        message: Message,
        enqueue_message_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
        let address = &message.onward_route.addresses[0].address;
        if let Some(connection) = self.connections.get_mut(&address.as_string()) {
            connection.process_message(message, enqueue_message_ref)?;
        } else {
            // todo - kick message back with error
            libc_println!(
                "ProcessMessage for TcpRouter, address {:?} not found",
                address
            );
        }
        Ok(true)
    }
}

impl Poll for TcpRouter {
    fn poll(
        &mut self,
        enqueue_message_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
        if matches!(self.listener, Some(_)) {
            self.accept_new_connections()?;
        }
        for (_, mut tcp_worker) in self.connections.iter_mut() {
            tcp_worker.poll(enqueue_message_ref.clone())?;
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
