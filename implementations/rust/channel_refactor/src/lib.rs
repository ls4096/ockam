use ockam_kex::error::KeyExchangeFailErrorKind;
use ockam_kex::xx::XXVault;
use ockam_kex::{CompletedKeyExchange, KeyExchanger, NewKeyExchanger};
use ockam_vault::types::PublicKey;
use ockam_vault::Secret;
use rand::{thread_rng, Rng};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::marker::PhantomData;
use ockam::message::{Address, Message};
use std::cell::RefCell;
use ockam_no_std_traits::{SecureChannelConnectCallback, Poll, EnqueueMessage, ProcessMessage};
use std::ops::Deref;

/// A channel address of zero indicates to the channel manager that
/// a new channel is being initiated
pub const CHANNEL_ZERO: &str = "00000000";

enum ExchangerRole {
    Initiator,
    Responder,
}

/// A Channel Manager creates secure channels on demand using the specified key exchange
/// generic. All keys will be created in the associated vault object
pub struct SecureChannel<
    I: KeyExchanger + 'static,
    R: KeyExchanger + 'static,
    E: NewKeyExchanger<I, R>,
> {
    vault: Arc<Mutex<dyn XXVault>>,
    new_key_exchanger: E,
    phantom_i: PhantomData<I>,
    phantom_r: PhantomData<R>,
    resp_key_ctx: Option<Arc<Box<dyn Secret>>>,
    init_key_ctx: Option<Arc<Box<dyn Secret>>>,
    transport: Address,
    callback: Rc<RefCell<dyn SecureChannelConnectCallback>>,
    address: Address,
    t: usize,
}

impl<I: KeyExchanger, R: KeyExchanger, E: NewKeyExchanger<I, R>> SecureChannel<I, R, E> {
    pub fn create(
        vault: Arc<Mutex<dyn XXVault>>,
        new_key_exchanger: E,
        resp_key_ctx: Option<Arc<Box<dyn Secret>>>,
        init_key_ctx: Option<Arc<Box<dyn Secret>>>,
        transport: Address,
        callback: Rc<RefCell<dyn SecureChannelConnectCallback>>,
    ) -> Result<Self, String> {
        let mut rng = rand::thread_rng();
        let y: f64 = rng.gen(); // generates a float between 0 and 1
        let y: u32 = (y*1000.00) as u32;
        let address = Address::WorkerAddress(y.to_le_bytes().to_vec());
        Ok(Self {
            vault,
            new_key_exchanger,
            phantom_i: PhantomData,
            phantom_r: PhantomData,
            resp_key_ctx,
            init_key_ctx,
            transport,
            callback,
            address,
            t: 0,
        })
    }

    pub fn address_as_string(&self) -> String {
        self.address.as_string()
    }
}

impl<I: KeyExchanger, R: KeyExchanger, E: NewKeyExchanger<I, R>> Poll for SecureChannel<I, R, E> {
    fn poll(&mut self, q_ref: Rc<RefCell<dyn EnqueueMessage>>) -> Result<bool, String> {
        if self.t == 0 {
            let cb = self.callback.clone();
            let mut cb = cb.deref().borrow_mut();
            cb.secure_channel_callback(self.address.clone());
            self.t += 1;
        }
        println!("channel poll");
        Ok(true)
    }
}

impl<I: KeyExchanger, R: KeyExchanger, E: NewKeyExchanger<I, R>> ProcessMessage for SecureChannel<I, R, E> {
    fn process_message(&mut self, message: Message, enqueue: Rc<RefCell<dyn EnqueueMessage>>) -> Result<bool, String> {
        println!("channel process message");
        Ok(true)
    }
}