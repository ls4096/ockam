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
    EnqueueMessage, Poll, ProcessMessage, SecureChannelConnectCallback, TransportListenCallback,
};
use ockam_tcp_router::tcp_router::TcpRouter;
use ockam_vault_software::DefaultVault;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread;
use ockam_channel_refactor::SecureChannel;

pub struct TestWorker {
    address: String,
    text: String,
    count: usize,
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
        enqueue_message_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
        let msg_text = "sent to you by TestWorker".as_bytes();
        let mut onward_addresses = Vec::new();

        onward_addresses
            .push(RouterAddress::worker_router_address_from_str("aabbccdd".into()).unwrap());
        let mut return_addresses = Vec::new();
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
        let mut q = enqueue_message_ref.deref().borrow_mut();
        q.enqueue_message(m)?;
        Ok(true)
    }
}

impl ProcessMessage for TestWorker {
    fn process_message(
        &mut self,
        message: Message,
        _q_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
        self.count += 1;
        if self.count > 3 {
            return Ok(false);
        }
        Ok(true)
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
        "aabbccdd".into(),
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
    local_address: Vec<u8>,
    channel_address: Option<Address>,
}

impl TransportListenCallback for TestTcpWorker {
    fn transport_listen_callback(
        &mut self,
        local_address: Address,
        peer_address: Address,
    ) -> Result<bool, String> {
        // create the vault and key exchanger
        let vault = Arc::new(Mutex::new(DefaultVault::default()));
        let attributes = SecretAttributes {
            stype: SecretType::Curve25519,
            persistence: SecretPersistence::Persistent,
            length: CURVE25519_SECRET_LENGTH,
        };
        let new_key_exchanger = XXNewKeyExchanger::new(
            CipherSuite::Curve25519AesGcmSha256,
            vault.clone(),
            vault.clone(),
        );
        Ok(true)
    }
}

impl SecureChannelConnectCallback for TestTcpWorker {
    fn secure_channel_callback(&mut self, address: Address) -> Result<bool, String> {
        println!("in channel callback");
        Ok(true)
    }
}

impl TestTcpWorker {
    pub fn new(
        node: Rc<RefCell<Node>>,
        is_initiator: bool,
        local_address: Vec<u8>,
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
        enqueue_message_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
        if self.count == 0 && self.is_initiator {
            let mut route = Route {
                addresses: vec![
                    RouterAddress::tcp_router_address_from_str("127.0.0.1:4052").unwrap(),
                    RouterAddress::worker_router_address_from_str("00112233").unwrap(),
                ],
            };
            let addr = Address::WorkerAddress(self.local_address.clone());
            let m = Message {
                onward_route: route,
                return_route: Route {
                    addresses: vec![RouterAddress::from_address(addr).unwrap()],
                },
                message_type: MessageType::Payload,
                message_body: "hello".as_bytes().to_vec(),
            };
            let mut q = enqueue_message_ref.deref().borrow_mut();
            q.enqueue_message(m)?;
        }
        self.count += 1;
        Ok(true)
    }
}

impl ProcessMessage for TestTcpWorker {
    fn process_message(
        &mut self,
        message: Message,
        enqueue_message_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
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
            let addr = Address::WorkerAddress(self.local_address.clone());
            let m = Message {
                onward_route: message.return_route.clone(),
                return_route: Route {
                    addresses: vec![RouterAddress::from_address(addr).unwrap()],
                },
                message_type: MessageType::Payload,
                message_body: "hello".as_bytes().to_vec(),
            };
            {
                let mut q = enqueue_message_ref.clone(); //rb
                let mut q = q.deref().borrow_mut();
                q.enqueue_message(m);
            }
            self.count += 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub fn responder_thread() {
    // create node
    let mut node = Rc::new(RefCell::new(Node::new("responder").unwrap()));

    // create test worker and register
    let worker_address = hex_vec_from_str("00112233".into()).unwrap();
    let worker = Rc::new(RefCell::new(
        (TestTcpWorker::new(node.clone(), false, worker_address.clone(), None)),
    ));
    let n = node.clone();
    let mut n = n.deref().borrow_mut();
    n.register_worker("00112233".to_string(), Some(worker.clone()), None);

    // create and register the transport
    let mut tcp_router = TcpRouter::new(Some("127.0.0.1:4052"), Some(worker.clone())).unwrap();
    let tcp_router = Rc::new(RefCell::new(tcp_router));
    n.register_transport(AddressType::Tcp, tcp_router.clone(), tcp_router.clone());

    n.run();
}

pub fn initiator_thread() {
    // give the responder time to spin up
    thread::sleep(time::Duration::from_millis(1000));

    // create the node
    let mut node = Rc::new(RefCell::new(Node::new("initiator").unwrap()));

    // create test worker and register
    let worker_address = hex_vec_from_str("aabbccdd".into()).unwrap();
    let worker = TestTcpWorker::new(
        node.clone(),
        true,
        worker_address,
        Some(Address::TcpAddress(
            SocketAddr::from_str("127.0.0.1:4052").unwrap(),
        )),
    );
    let worker_ref = Rc::new(RefCell::new(worker));
    let mut n = node.deref().borrow_mut();
    n.register_worker(
        "aabbccdd".to_string(),
        Some(worker_ref.clone()),
        Some(worker_ref.clone()),
    );

    // create and register the transport
    let mut tcp_router = TcpRouter::new(None, None).unwrap();
    let tcp_router = Rc::new(RefCell::new(tcp_router));
    n.register_transport(AddressType::Tcp, tcp_router.clone(), tcp_router.clone());

    // create connection
    let tcp_connection = {
        let tcp = tcp_router.clone();
        let mut tcp = tcp.deref().borrow_mut();
        let tcp_connection = tcp.try_connect("127.0.0.1:4052", Some(500)).unwrap();
        tcp_connection
    };

    // create vault, key exchanger, and request channel
    // create the vault and key exchanger
    let vault = Arc::new(Mutex::new(DefaultVault::default()));
    let attributes = SecretAttributes {
        stype: SecretType::Curve25519,
        persistence: SecretPersistence::Persistent,
        length: CURVE25519_SECRET_LENGTH,
    };
    let new_key_exchanger = XXNewKeyExchanger::new(
        CipherSuite::Curve25519AesGcmSha256,
        vault.clone(),
        vault.clone(),
    );
    let channel = SecureChannel::create(
        vault,
        new_key_exchanger,
        None,
        None,
        tcp_connection,
        worker_ref.clone(),
    ).unwrap();
    let channel_address = channel.address_as_string();
    let channel_ref = Rc::new(RefCell::new(channel));
    let ch = channel_ref.clone();

    n.register_worker(channel_address, Some(ch.clone()), Some(ch.clone()));

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
