extern crate alloc;
use alloc::collections::VecDeque;
use alloc::rc::Rc;

use alloc::string::String;
use core::cell::RefCell;
use core::ops::Deref;
use core::time;
use ockam::message::{Address, AddressType, Message};
use ockam::vault::types::{
    SecretAttributes, SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH,
};
use ockam_message_router::MessageRouter;
use ockam_no_std_traits::{PollHandle, ProcessMessageHandle, SecureChannelConnectCallback, TransportListenCallback, EnqueueMessage};
use ockam_queue::Queue;
use ockam_tcp_router::tcp_router::TcpRouter;
use ockam_vault_software::DefaultVault;
use ockam_worker_manager::WorkerManager;
use std::sync::{Arc, Mutex};
use std::thread;

pub enum Transport {
    Tcp(Rc<TcpRouter>),
}

pub struct Node {
    message_queue: Rc<RefCell<Queue<Message>>>,
    message_router: MessageRouter,
    worker_manager: Rc<RefCell<WorkerManager>>,
    modules_to_poll: VecDeque<PollHandle>,
    transports: VecDeque<Rc<RefCell<Transport>>>,
    _role: String,
}

impl Node {
    pub fn new(role: &str) -> Result<Self, String> {
        Ok(Node {
            message_queue: Rc::new(RefCell::new(Queue::new())),
            message_router: MessageRouter::new().unwrap(),
            worker_manager: Rc::new(RefCell::new(WorkerManager::new())),
            modules_to_poll: VecDeque::new(),
            transports: VecDeque::new(),
            _role: role.to_string(),
        })
    }

    pub fn create_secure_channel(
        &mut self,
        route: Vec<Address>,
        callback: Rc<dyn SecureChannelConnectCallback>,
    ) -> Result<bool, String> {
        Ok(true)
    }

    pub fn register_worker(
        &mut self,
        address: Vec<u8>,
        message_handler: Option<ProcessMessageHandle>,
        poll_handler: Option<PollHandle>,
    ) -> Result<bool, String> {
        let mut wm = self.worker_manager.deref().borrow_mut();
        wm.register_worker(address, message_handler, poll_handler)
    }

    pub fn register_transport(
        &mut self,
        address_type: AddressType,
        pmh: ProcessMessageHandle,
        ph: PollHandle,
    ) {
        println!("registering transport");
        self.message_router
            .register_address_type_handler(address_type, pmh.clone());
        self.modules_to_poll.push_back(ph.clone());
    }

    pub fn get_enqueue_ref(&self) -> Result<Rc<RefCell<dyn EnqueueMessage>>, String> {
        Ok(self.message_queue.clone())
    }

    pub fn run(&mut self) -> Result<(), String> {
        self.message_router
            .register_address_type_handler(AddressType::Worker, self.worker_manager.clone())?;
        self.modules_to_poll.push_back(self.worker_manager.clone());

        let mut stop = false;
        loop {
            match self.message_router.poll(self.message_queue.clone()) {
                Ok(keep_going) => {
                    if !keep_going {
                        break;
                    }
                }
                Err(s) => {
                    return Err(s);
                }
            }
            for p_ref in self.modules_to_poll.iter() {
                let p = p_ref.clone();
                let mut p = p.deref().borrow_mut();
                match p.poll(self.message_queue.clone()) {
                    Ok(keep_going) => {
                        if !keep_going {
                            stop = true;
                            break;
                        }
                    }
                    Err(s) => {
                        return Err(s);
                    }
                }
            }
            if stop {
                break;
            }
            thread::sleep(time::Duration::from_millis(100));
        }
        Ok(())
    }
}
