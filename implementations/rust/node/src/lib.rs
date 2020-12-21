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
use ockam_no_std_traits::{
    PollHandle, ProcessMessageHandle, SecureChannelConnectCallback, WorkerRegistration,
};
use ockam_tcp_router::tcp_router::TcpRouter;
use ockam_vault_software::DefaultVault;
use std::sync::{Arc, Mutex};
use std::thread;

pub enum Transport {
    Tcp(Rc<TcpRouter>),
}

pub struct Node {
    message_router: MessageRouter,
    modules_to_poll: VecDeque<PollHandle>,
    _role: String,
}

impl Node {
    pub fn new(role: &str) -> Result<Self, String> {
        Ok(Node {
            message_router: MessageRouter::new().unwrap(),
            modules_to_poll: VecDeque::new(),
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
        address: Address,
        message_handler: Option<ProcessMessageHandle>,
        poll_handler: Option<PollHandle>,
    ) -> Result<bool, String> {
        println!("registering {}", address.as_string());
        if let Some(mh) = message_handler {
            self.message_router
                .register_message_handler(address, mh.clone());
        }
        if let Some(ph) = poll_handler {
            self.modules_to_poll.push_back(ph.clone());
        }
        Ok(true)
    }

    pub fn run(&mut self) -> Result<(), String> {
        let mut stop = false;
        loop {
            let mut new_workers: Vec<WorkerRegistration> = Vec::new();
            let mut new_messages: Vec<Message> = Vec::new();
            match self.message_router.poll() {
                Ok((keep_going, _, workers_opt)) => {
                    if !keep_going {
                        break;
                    }
                    if let Some(mut workers) = workers_opt {
                        new_workers.append(&mut workers);
                    }
                }
                Err(s) => {
                    return Err(s);
                }
            }
            for p_ref in self.modules_to_poll.iter() {
                let p = p_ref.clone();
                let mut p = p.deref().borrow_mut();
                match p.poll() {
                    Ok((keep_going, messages_opt, workers_opt)) => {
                        if !keep_going {
                            break;
                        }
                        if let Some(mut workers) = workers_opt {
                            new_workers.append(&mut workers);
                        }
                        if let Some(mut messages) = messages_opt {
                            new_messages.append(&mut messages)
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
            for w in new_workers {
                if let Some(mh) = w.message_processor {
                    self.message_router
                        .register_message_handler(w.address, mh.clone());
                }
                if let Some(p) = w.poll {
                    self.modules_to_poll.push_back(p.clone());
                }
            }
            self.message_router.enqueue_messages(new_messages);
            thread::sleep(time::Duration::from_millis(100));
        }
        Ok(())
    }
}
