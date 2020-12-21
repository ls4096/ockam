#![no_std]
extern crate alloc;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use ockam::message::{Address, Message};

/// ProcessMessage trait is for workers to process messages addressed to them
///
/// A worker registers its address along with a ProcessMessage trait. The WorkerManager
/// will then call the ProcessMessage trait when the next onward_route address is that of
/// the worker.
pub trait ProcessMessage {
    fn process_message(
        &mut self,
        message: Message, //todo - add context
    ) -> Result<(bool, Option<Vec<Message>>, Option<Vec<WorkerRegistration>>), String>;
}
pub type ProcessMessageHandle = Rc<RefCell<dyn ProcessMessage>>;

/// Poll trait is for workers to get cpu cycles on a regular basis.
///
/// A worker gets polled by registering its address and poll trait with the Node.
/// poll() will be called once each polling interval.
pub trait Poll {
    //todo - add context
    fn poll(
        &mut self,
    ) -> Result<(bool, Option<Vec<Message>>, Option<Vec<WorkerRegistration>>), String>;
}
pub type PollHandle = Rc<RefCell<dyn Poll>>;

pub trait TransportListenCallback {
    fn transport_listen_callback(
        &mut self,
        local_address: Address,
        peer_address: Address,
    ) -> Result<(bool, Option<Vec<Message>>, Option<Vec<WorkerRegistration>>), String>;
}

pub trait SecureChannelConnectCallback {
    fn secure_channel_callback(&mut self, address: Address) -> Result<bool, String>;
}

pub struct WorkerRegistration {
    pub address: Address,
    pub message_processor: Option<ProcessMessageHandle>,
    pub poll: Option<PollHandle>,
}
