#![no_std]
extern crate alloc;
use alloc::rc::Rc;
use alloc::string::String;
use core::cell::RefCell;
use ockam::message::Message;
use ockam_queue::Queue;

/// ProcessMessage trait is for workers to process messages addressed to them
///
/// A worker registers its address along with a ProcessMessage trait. The WorkerManager
/// will then call the ProcessMessage trait when the next onward_route address is that of
/// the worker.
pub trait ProcessMessage {
    fn process_message(
        &mut self,
        message: Message, //todo - add context
        enqueue: Rc<RefCell<dyn Queue<Message>>>,
    ) -> Result<bool, String>;
}
pub type ProcessMessageHandle = Rc<RefCell<dyn ProcessMessage>>;

/// poll trait is for workers to get cpu cycles on a regular basis.
///
/// A worker gets polled by registering its address and poll trait with the Node.
/// poll() will be called once each polling interval.
pub trait Poll {
    //todo - add context
    fn poll(&mut self, q_ref: Rc<RefCell<dyn Queue<Message>>>) -> Result<bool, String>;
}
pub type PollHandle = Rc<RefCell<dyn Poll>>;

