#![allow(unused)]
use ockam_node::Node;
extern crate alloc;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::*;
use core::cell::RefCell;
use core::ops::Deref;
use core::time;
use ockam::kex::xx::XXNewKeyExchanger;
use ockam::kex::CipherSuite;
use ockam::message::{
    hex_vec_from_str, Address, AddressType, Message, MessageType, Route, RouterAddress,
};
use ockam::vault::types::{
    SecretAttributes, SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH,
};
use ockam_no_std_traits::{
    Poll, ProcessMessage, SecureChannelConnectCallback, TransportListenCallback, WorkerRegistration,
};
use ockam_tcp_router::tcp_router::TcpRouter;
use ockam_vault_software::DefaultVault;
use std::any::Any;
use std::net::SocketAddr;
use std::str::*;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct TestWorker {
    address: String,
    text: String,
    count: usize,
}

enum State {
    SendM1,
    ReceiveM1,
}

impl TestWorker {
    pub fn new(address: String, text: String) -> Self {
        TestWorker {
            address,
            text,
            count: 0,
        }
    }
}

impl Poll for TestWorker {
    fn poll(
        &mut self,
    ) -> Result<(bool, Option<Vec<Message>>, Option<Vec<WorkerRegistration>>), String> {
        let msg_text = "sent to you by TestWorker".as_bytes();
        let mut onward_addresses = Vec::new();

        onward_addresses
            .push(RouterAddress::worker_router_address_from_str("aabbccdd".into()).unwrap());
        let mut return_addresses: Vec<RouterAddress> = Vec::new();
        return_addresses
            .push(RouterAddress::worker_router_address_from_str(&self.address).unwrap());
        let m = Message {
            onward_route: Route {
                addresses: onward_addresses,
            },
            return_route: Route {
                addresses: return_addresses,
            },
            message_type: MessageType::Payload,
            message_body: msg_text.to_vec(),
        };
        Ok((true, Some(vec![m]), None))
    }
}

impl ProcessMessage for TestWorker {
    fn process_message(
        &mut self,
        message: Message,
    ) -> Result<(bool, Option<Vec<Message>>, Option<Vec<WorkerRegistration>>), String> {
        self.count += 1;
        if self.count > 10 {
            return Ok((false, None, None));
        }
        Ok((true, None, None))
    }
}

#[test]
fn test_node() {
    // create node
    let mut node = Node::new("").unwrap();
    // Now create the worker(s) and register them with the worker manager
    let test_worker = Rc::new(RefCell::new(TestWorker::new(
        "aabbccdd".into(),
        "text".into(),
    )));

    node.register_worker(
        Address::WorkerAddress("aabbccdd".as_bytes().to_vec()),
        Some(test_worker.clone()),
        Some(test_worker.clone()),
    )
    .expect("failed to register worker");

    if let Err(_s) = node.run() {
        assert!(false);
    } else {
        assert!(true);
    }
}

pub struct TestTcpWorker {
    node: Rc<RefCell<Node>>,
    is_initiator: bool,
    count: usize,
    remote: Address,
    local_address: Address,
    channel_address: Option<Address>,
}

impl TransportListenCallback for TestTcpWorker {
    fn transport_listen_callback(
        &mut self,
        local_address: Address,
        peer_address: Address,
    ) -> Result<(bool, Option<Vec<Message>>, Option<Vec<WorkerRegistration>>), String> {
        // create the vault and key exchanger

        Ok((true, None, None))
    }
}

// impl SecureChannelConnectCallback for TestTcpWorker {
//     fn secure_channel_callback(&mut self, address: Address) -> Result<bool, String> {
//         println!("in channel callback");
//         Ok(true)
//     }
// }

impl TestTcpWorker {
    pub fn new(
        node: Rc<RefCell<Node>>,
        is_initiator: bool,
        local_address: Address,
        opt_remote: Option<Address>,
    ) -> Self {
        if let Some(r) = opt_remote {
            TestTcpWorker {
                node,
                is_initiator,
                count: 0,
                remote: r,
                local_address,
                channel_address: None,
            }
        } else {
            TestTcpWorker {
                node,
                is_initiator,
                count: 0,
                remote: Address::TcpAddress(SocketAddr::from_str("127.0.0.1:4050").unwrap()),
                local_address,
                channel_address: None,
            }
        }
    }
}

impl Poll for TestTcpWorker {
    fn poll(
        &mut self,
    ) -> Result<(bool, Option<Vec<Message>>, Option<Vec<WorkerRegistration>>), String> {
        if self.count == 0 && self.is_initiator {
            self.count += 1;
            println!("in initiator poll for TestTcpWorker");
            let mut route = Route {
                addresses: vec![
                    RouterAddress::tcp_router_address_from_str("127.0.0.1:4052").unwrap(),
                    RouterAddress::worker_router_address_from_str("00112233").unwrap(),
                ],
            };
            let m = Message {
                onward_route: route,
                return_route: Route {
                    addresses: vec![
                        RouterAddress::from_address(self.local_address.clone()).unwrap()
                    ],
                },
                message_type: MessageType::Payload,
                message_body: "hello".as_bytes().to_vec(),
            };
            return Ok((true, Some(vec![m]), None));
        }
        Ok((true, None, None))
    }
}

impl ProcessMessage for TestTcpWorker {
    fn process_message(
        &mut self,
        message: Message,
    ) -> Result<(bool, Option<Vec<Message>>, Option<Vec<WorkerRegistration>>), String> {
        if self.is_initiator {
            println!(
                "Initiator: message received: {}",
                String::from_utf8(message.message_body).unwrap()
            );
        } else {
            println!(
                "Responder: message received: {}",
                String::from_utf8(message.message_body).unwrap()
            );
        }
        if self.count < 5 {
            let m = Message {
                onward_route: message.return_route.clone(),
                return_route: Route {
                    addresses: vec![
                        RouterAddress::from_address(self.local_address.clone()).unwrap()
                    ],
                },
                message_type: MessageType::Payload,
                message_body: "hello".as_bytes().to_vec(),
            };
            self.count += 1;
            Ok((true, Some(vec![m]), None))
        } else {
            Ok((false, None, None))
        }
    }
}

pub fn responder_thread() {
    // create node
    let mut node = Rc::new(RefCell::new(Node::new("responder").unwrap()));

    // create test worker and register
    let worker_address = Address::WorkerAddress("00112233".as_bytes().to_vec());
    let worker_ref = Rc::new(RefCell::new(
        (TestTcpWorker::new(node.clone(), false, worker_address.clone(), None)),
    ));
    let n = node.clone();
    let mut n = n.deref().borrow_mut();
    n.register_worker(worker_address.clone(), Some(worker_ref.clone()), None);

    // create transport and register
    let mut tcp_router_ref = Rc::new(RefCell::new(
        TcpRouter::new(Some("127.0.0.1:4052"), Some(worker_ref.clone()))
            .expect("responder failed to create tcp_router"),
    ));
    {
        let tcp_router = tcp_router_ref.clone();
        let mut tcp_router = tcp_router_ref.deref().borrow_mut();

        n.register_worker(
            tcp_router.address(),
            Some(tcp_router_ref.clone()),
            Some(tcp_router_ref.clone()),
        );
    }

    println!("Responder node running");
    n.run();
}

pub fn initiator_thread() {
    // give the responder time to spin up
    thread::sleep(time::Duration::from_millis(1000));

    // create the node
    let mut node_ref = Rc::new(RefCell::new(Node::new("initiator").unwrap()));

    // create test worker and register
    let worker_address = Address::WorkerAddress("aabbccdd".as_bytes().to_vec());
    let worker = TestTcpWorker::new(
        node_ref.clone(),
        true,
        worker_address.clone(),
        Some(Address::TcpAddress(
            SocketAddr::from_str("127.0.0.1:4052").unwrap(),
        )),
    );
    let worker_ref = Rc::new(RefCell::new(worker));
    let mut n = node_ref.clone();
    let mut n = n.deref().borrow_mut();
    n.register_worker(
        worker_address,
        Some(worker_ref.clone()),
        Some(worker_ref.clone()),
    );

    // create the transport and register
    let mut tcp_router_ref = Rc::new(RefCell::new(
        TcpRouter::new(None, None).expect("initiator failed to create tcp_router"),
    ));
    {
        let tcp_router = tcp_router_ref.clone();
        let mut tcp_router = tcp_router.deref().borrow_mut();
        n.register_worker(
            tcp_router.address(),
            Some(tcp_router_ref.clone()),
            Some(tcp_router_ref.clone()),
        );
        // create connection
        let tcp_connection = tcp_router
            .try_connect("127.0.0.1:4052", Some(500))
            .expect("tcp.try_connect failed");
        println!("tcp connection established");
    }
    println!("initiator node running");
    n.run();
    return;
}

#[test]
fn test_tcp() {
    // spin up responder (listener) and initiator (client) threads
    let responder_handle = thread::spawn(|| responder_thread());
    let initiator_handle = thread::spawn(|| initiator_thread());
    match initiator_handle.join() {
        Ok(()) => {
            println!("initiator joined");
        }
        Err(_) => {
            assert!(false);
        }
    }
}
