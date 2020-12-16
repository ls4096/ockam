#![allow(unused)]
#![no_std]
extern crate alloc;

use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::string::String;
use core::cell::{RefCell, RefMut};
use core::ops::Deref;

use libc_print::*;

use ockam::message::{AddressType, Message};
use ockam_no_std_traits::{Poll, ProcessMessage, ProcessMessageHandle};
use ockam_queue::{Enqueue, Queue};

pub struct MessageRouter {
    handlers: [Option<ProcessMessageHandle>; 256],
}

const INIT_TO_NO_RECORD: Option<ProcessMessageHandle> = None;

impl MessageRouter {
    pub fn new() -> Result<Self, String> {
        Ok(MessageRouter {
            handlers: [INIT_TO_NO_RECORD; 256],
        })
    }

    pub fn register_address_type_handler(
        &mut self,
        address_type: AddressType,
        handler: ProcessMessageHandle,
    ) -> Result<bool, String> {
        self.handlers[address_type as usize] = Some(handler);
        Ok(true)
    }

    pub fn poll(
        &self,
        mut queue_ref: Rc<RefCell<dyn Queue<Message>>>,
    ) -> Result<bool, String> {
        loop {
            {
                let message: Option<Message> = {
                    let mut qr = queue_ref.clone();
                    let mut queue = qr.deref().borrow_mut();
                    queue.dequeue()
                };
                match message {
                    Some(m) => {
                        let address_type = m.onward_route.addresses[0].a_type as usize;
                        let maybe_handler : &Option<ProcessMessageHandle> =  &self.handlers[address_type];
                        match maybe_handler {
                            Some(h) => {
                                let handler = h.clone();
                                let mut handler = handler.deref().borrow_mut();

                                match handler.process_message(m, queue_ref.clone()) {
                                    Ok(keep_going) => {
                                        if !keep_going {
                                            return Ok(false);
                                        }
                                    }
                                    Err(s) => {
                                        return Err(s);
                                    }
                                }
                            }
                            None => {
                                return Err("no handler for message type".into());
                            }
                        }
                    }
                    None => {
                        break;
                    }
                }
            }
        }
        Ok(true)
    }
}
