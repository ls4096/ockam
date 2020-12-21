#![allow(unused)]

extern crate alloc;

use crate::tcp_worker::TcpWorker;
use alloc::rc::Rc;
use libc_print::*;
use ockam::message::Address::TcpAddress;
use ockam::message::MAX_MESSAGE_SIZE;
use ockam::message::{Address, Message};
use ockam_no_std_traits::{Poll, ProcessMessage, TransportListenCallback, WorkerRegistration};
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
    address: Address,
}

impl TcpRouter {
    pub fn new(
        listen_addr: Option<&str>,
        connect_callback: Option<Rc<RefCell<dyn TransportListenCallback>>>,
    ) -> Result<Self, String> {
        let connections: HashMap<String, TcpWorker> = HashMap::new();
        let address = TcpAddress(
            SocketAddr::from_str("0.0.0.0:0").expect("invalid socket address in TcpRouter::new"),
        );
        return match listen_addr {
            Some(la) => {
                if let Ok(l) = TcpListener::bind(la) {
                    l.set_nonblocking(true).unwrap();
                    Ok(TcpRouter {
                        connections,
                        listener: Some(l),
                        address,
                    })
                } else {
                    Err("failed to bind tcp listener".into())
                }
            }
            None => Ok(TcpRouter {
                connections,
                listener: None,
                address,
            }),
        };
    }

    fn accept_new_connections(&mut self) -> Result<bool, String> {
        let mut keep_going = true;
        let mut new_connections: Vec<WorkerRegistration> = Vec::new();
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

    pub fn address_as_string(&self) -> String {
        self.address.as_string()
    }
    pub fn address(&self) -> Address {
        self.address.clone()
    }
}

impl ProcessMessage for TcpRouter {
    fn process_message(
        &mut self,
        message: Message,
    ) -> Result<(bool, Option<Vec<Message>>, Option<Vec<WorkerRegistration>>), String> {
        let address = &message.onward_route.addresses[0].address;
        return if let Some(connection) = self.connections.get_mut(&address.as_string()) {
            connection.process_message(message)
        } else {
            // todo - kick message back with error
            libc_println!(
                "ProcessMessage for TcpRouter, address {:?} not found",
                address
            );
            Err(format!(
                "ProcessMessage for TcpRouter, address {:?} not found",
                address
            ))
        };
    }
}

impl Poll for TcpRouter {
    fn poll(
        &mut self,
    ) -> Result<(bool, Option<Vec<Message>>, Option<Vec<WorkerRegistration>>), String> {
        if matches!(self.listener, Some(_)) {
            self.accept_new_connections()
                .expect("failed accept_new_connections");
        }
        let mut messages: Vec<Message> = Vec::new();
        let mut workers: Vec<WorkerRegistration> = Vec::new();
        let mut status = true;
        for (_, mut tcp_worker) in self.connections.iter_mut() {
            let (s, m, w) = tcp_worker.poll()?;
            if !s {
                status = false;
            }
            if let Some(mut ms) = m {
                messages.append(&mut ms);
            }
            if let Some(mut ws) = w {
                workers.append(&mut ws);
            }
        }
        let messages = if messages.is_empty() {
            None
        } else {
            Some(messages)
        };
        let workers = if workers.is_empty() {
            None
        } else {
            Some(workers)
        };
        Ok((status, messages, workers))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
