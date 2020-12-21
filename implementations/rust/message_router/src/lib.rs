#![allow(unused)]
#![no_std]
extern crate alloc;
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::ops::Deref;
use hashbrown::HashMap;
use libc_print::*;
use ockam::message::Address::WorkerAddress;
use ockam::message::{Address, AddressType, Message, Route, RouterAddress};
use ockam::system::commands::WorkerCommand::AddLine;
use ockam_no_std_traits::{Poll, ProcessMessage, ProcessMessageHandle, WorkerRegistration};

pub struct MessageRouter {
    message_queue: VecDeque<Message>,
    handlers: HashMap<Vec<u8>, ProcessMessageHandle>,
}

impl MessageRouter {
    pub fn new() -> Result<Self, String> {
        Ok(MessageRouter {
            message_queue: VecDeque::new(),
            handlers: HashMap::new(),
        })
    }

    pub fn enqueue_messages(&mut self, mut messages: Vec<Message>) -> Result<bool, String> {
        self.message_queue.append(&mut messages.into());
        Ok(true)
    }

    pub fn register_message_handler(
        &mut self,
        address: Address,
        handler: ProcessMessageHandle,
    ) -> Result<bool, String> {
        self.handlers
            .insert(Vec::from(address.as_string()), handler.clone());
        Ok(true)
    }

    pub fn poll(
        &mut self,
    ) -> Result<(bool, Option<Vec<Message>>, Option<Vec<WorkerRegistration>>), String> {
        let mut keep_going = true;
        let mut messages: Vec<Message> = Vec::new();
        let mut worker_registrations: Vec<WorkerRegistration> = Vec::new();
        loop {
            let mut message: Option<Message> = self.message_queue.remove(0);
            match message {
                Some(mut m) => {
                    if m.onward_route.addresses.len() == 0 {
                        return Err("No route supplied".into());
                    }
                    let mut address = m.onward_route.addresses[0].address.as_string();

                    // todo this is special-case handling for the TcpRouter (obviously).
                    // The TCP router (there is only ever one per node) always has
                    // the 0.0.0.0:0 address and it handles all tcp addresses.
                    if matches!(m.onward_route.addresses[0].address, Address::TcpAddress(a)) {
                        address = "0.0.0.0:0".to_string();
                    }

                    match self.handlers.get_mut(&address.as_bytes().to_vec()) {
                        Some(h) => {
                            let handler = h.clone();
                            let mut handler = handler.deref().borrow_mut();
                            let (status, new_messages_opt, new_workers_opt) =
                                handler.process_message(m)?;
                            if let Some(mut m) = new_messages_opt {
                                messages.append(&mut m);
                            }
                            if let Some(mut w) = new_workers_opt {
                                worker_registrations.append(&mut w);
                            }
                            if !status {
                                keep_going = false;
                                break;
                            }
                        }
                        None => {
                            return Err("no handler for message type".into());
                        }
                    };
                }
                None => {
                    break;
                }
            }
        }
        self.message_queue.append(&mut messages.into());
        Ok((keep_going, None, Some(worker_registrations)))
    }
}
