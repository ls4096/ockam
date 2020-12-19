use ockam::message::Address::WorkerAddress;
use ockam::message::{Address, Message, MessageType, Route, RouterAddress};
use ockam_kex::error::KeyExchangeFailErrorKind;
use ockam_kex::xx::XXVault;
use ockam_kex::{CompletedKeyExchange, KeyExchanger, NewKeyExchanger};
use ockam_no_std_traits::{EnqueueMessage, Poll, ProcessMessage, SecureChannelConnectCallback};
use ockam_vault::types::PublicKey;
use ockam_vault::Secret;
use rand::{thread_rng, Rng};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use ockam::message::MessageType::{KeyAgreementM2, KeyAgreementM3};

/// A channel address of zero indicates to the channel manager that
/// a new channel is being initiated
pub const CHANNEL_ZERO: &str = "00000000";

enum State {
    SendM1,
    ReceiveM1,
    SendM2,
    ReceiveM2,
    SendM3,
    ReceiveM3,
    Done,
}

fn send_m1(q_ref: Rc<RefCell<dyn EnqueueMessage>>, route: Route, channel_addr: Address) -> Result<bool, String> {
    let m = Message {
        onward_route: route,
        return_route: Route {
            addresses: vec![RouterAddress::from_address(channel_addr).unwrap()],
        },
        message_type: MessageType::KeyAgreementM1,
        message_body: "M1".as_bytes().to_vec(),
    };
    println!("Sending m1");
    m.onward_route.print_route();
    m.return_route.print_route();
    let mut q = q_ref.deref().borrow_mut();
    q.enqueue_message(m)
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
    route: Route,
    callback: Rc<RefCell<dyn SecureChannelConnectCallback>>,
    address: Address,
    state: State,
    t: usize,
}

impl<I: KeyExchanger, R: KeyExchanger, E: NewKeyExchanger<I, R>> SecureChannel<I, R, E> {

    pub fn create(
        vault: Arc<Mutex<dyn XXVault>>,
        new_key_exchanger: E,
        resp_key_ctx: Option<Arc<Box<dyn Secret>>>,
        init_key_ctx: Option<Arc<Box<dyn Secret>>>,
        route: Route,
        callback: Rc<RefCell<dyn SecureChannelConnectCallback>>,
        q_ref: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<Self, String> {
        let mut rng = rand::thread_rng();
        let y: f64 = rng.gen(); // generates a float between 0 and 1
        let y: u32 = (y * 1000.00) as u32;
        let address = Address::WorkerAddress(y.to_le_bytes().to_vec());

        send_m1(q_ref, route.clone(), address.clone());

        Ok(Self {
            vault,
            new_key_exchanger,
            phantom_i: PhantomData,
            phantom_r: PhantomData,
            resp_key_ctx,
            init_key_ctx,
            route,
            callback,
            address,
            t: 0,
            state: State::ReceiveM2,
        })
    }

    pub fn address_as_string(&self) -> String {
        self.address.as_string()
    }

    pub fn address_as_u8(&self) -> Vec<u8> {
        match self.address.clone() {
            Address::WorkerAddress(u) => u,
            _ => vec![],
        }
    }

    fn receive_m1(&mut self, m1: Message, q_ref: Rc<RefCell<dyn EnqueueMessage>>) -> Result<bool, String> {
        self.route = m1.return_route.clone();
        let mut q = q_ref.deref().borrow_mut();
        self.state = State::ReceiveM3;
        q.enqueue_message( Message {
            onward_route: self.route.clone(),
            return_route: Route{ addresses: vec![RouterAddress::from_address(self.address.clone()).unwrap()]},
            message_type: KeyAgreementM2,
            message_body: "M2".as_bytes().to_vec()
        })
    }

    fn receive_m2(&mut self, m2: Message, q_ref: Rc<RefCell<dyn EnqueueMessage>>) -> Result<bool, String> {
        self.route = m2.return_route.clone();
        let mut q = q_ref.deref().borrow_mut();
        self.state = State::Done;
        q.enqueue_message( Message {
            onward_route: self.route.clone(),
            return_route: Route{ addresses: vec![RouterAddress::from_address(self.address.clone()).unwrap()]},
            message_type: KeyAgreementM3,
            message_body: "M2".as_bytes().to_vec()
        });

        Ok(true)
    }

    fn receive_m3(&mut self, m3: Message, q_ref: Rc<RefCell<dyn EnqueueMessage>>) -> Result<bool, String> {
        self.route = m3.return_route.clone();
        let mut q = q_ref.deref().borrow_mut();
        self.state = State::Done;
        q.enqueue_message( Message {
            onward_route: self.route.clone(),
            return_route: Route{ addresses: vec![RouterAddress::from_address(self.address.clone()).unwrap()]},
            message_type: KeyAgreementM2,
            message_body: "M3".as_bytes().to_vec()
        })
    }

    fn process_kex(&mut self, q_ref: Rc<RefCell<dyn EnqueueMessage>>) -> Result<bool, String> {
        match self.state {
            //State::SendM1 => self.send_m1(q_ref),
            _ => Ok(true),
        }
    }

    pub fn listen(
        vault: Arc<Mutex<dyn XXVault>>,
        new_key_exchanger: E,
        resp_key_ctx: Option<Arc<Box<dyn Secret>>>,
        init_key_ctx: Option<Arc<Box<dyn Secret>>>,
        callback: Rc<RefCell<dyn SecureChannelConnectCallback>>,
    ) -> Result<Self, String> {
        let mut rng = rand::thread_rng();
        let y: f64 = rng.gen(); // generates a float between 0 and 1
        let y: u32 = (y * 1000.00) as u32;
        let address = Address::WorkerAddress(y.to_le_bytes().to_vec());

        Ok(Self {
            vault,
            new_key_exchanger,
            phantom_i: PhantomData,
            phantom_r: PhantomData,
            resp_key_ctx,
            init_key_ctx,
            route: Route{addresses:vec![]},
            callback,
            address,
            t: 0,
            state: State::ReceiveM1,
        })
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
        match self.state {
            State::Done => Ok(true),
            _ => self.process_kex(q_ref),
        };
        Ok(true)
    }
}

impl<I: KeyExchanger, R: KeyExchanger, E: NewKeyExchanger<I, R>> ProcessMessage
    for SecureChannel<I, R, E>
{
    fn process_message(
        &mut self,
        message: Message,
        enqueue: Rc<RefCell<dyn EnqueueMessage>>,
    ) -> Result<bool, String> {
        return match self.state {
            State::Done => {
                println!("decrypt and forward");
                Ok(true)
            }
            State::ReceiveM2 => {
                self.receive_m2(message, enqueue)?;
                Ok(true)
                //todo - callback
            }
            State::ReceiveM3 => {
                self.receive_m3(message, enqueue)?;
                Ok(true)
                //todo - callback
            }
            _ => Err("state error".into())
        };
        Ok(true)
    }
}
