use std::str;
use std::sync::{mpsc, Arc, Mutex};

use crate::log::log_error;
use ockam::kex::CipherSuite;
use ockam::message::Address::ChannelAddress;
use ockam::message::{Address, Route, RouterAddress, Codec};
use ockam::secure_channel::ChannelManager;
use ockam::system::commands::{ChannelCommand, OckamCommand, WorkerCommand, RouterCommand};
use ockam::vault::types::{
    PublicKey, SecretAttributes, SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH,
};
use ockam::vault::Secret;
use ockam_kex_xx::{XXInitiator, XXNewKeyExchanger, XXResponder, XXVault};
use ockam_node::Node;
use std::sync::mpsc::{SendError, Sender, Receiver};

type XXChannelManager = ChannelManager<XXInitiator, XXResponder, XXNewKeyExchanger>;

pub struct Actor {
    vault: Arc<Mutex<dyn XXVault + Send>>,
    address: Address,
    secret: Option<Arc<Box<dyn Secret>>>,
    channel_manager: Option<ChannelManager<XXInitiator, XXResponder, XXNewKeyExchanger>>,
    channel_manager_tx: Option<Sender<OckamCommand>>,
    router_rx: Option<Receiver<OckamCommand>>,
    router_tx: Option<Sender<OckamCommand>>,
    node: Node,
}

impl From<&dyn Addressable> for String {
    fn from(a: &dyn Addressable) -> Self {
        a.address().as_string()
    }
}

pub trait Addressable {
    fn address(&self) -> Address;
}

impl Addressable for Actor {
    fn address(&self) -> Address {
        self.address.clone()
    }
}

impl Actor {
    pub fn secret_attributes(&self) -> SecretAttributes {
        SecretAttributes {
            stype: SecretType::Curve25519,
            persistence: SecretPersistence::Persistent,
            length: CURVE25519_SECRET_LENGTH,
        }
    }

    pub fn set_secret(&mut self, secret: Box<dyn Secret>) {
        self.secret = Some(Arc::new(secret));
    }

    pub fn secret(&self) -> Option<Arc<Box<dyn Secret>>> {
        match &self.secret {
            Some(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn create_client_transport(&mut self) {
        self.create_transport(None);
    }

    pub fn create_server_transport(&mut self, listen_address: &str) {
        self.create_transport(Some(listen_address));
    }

    fn create_transport(&mut self, listen_address: Option<&str>) {
        match self.node.initialize_transport(listen_address) {
            Err(e) => panic!(e),
            _ => (),
        }
    }

    pub fn default_new_key_exchanger(&self) -> XXNewKeyExchanger {
        XXNewKeyExchanger::new(
            CipherSuite::Curve25519AesGcmSha256,
            self.vault.clone(),
            self.vault.clone(),
        )
    }

    pub fn create_channel_manager(&mut self) {
        let (channel_tx, channel_rx) = mpsc::channel();

        match &self.router_tx {
            Some(rtx) => {
                let channel_manager = XXChannelManager::new(
                    channel_rx,
                    channel_tx.clone(),
                    rtx.clone(),
                    self.vault.clone(),
                    self.default_new_key_exchanger(),
                    self.secret(),
                    None,
                )
                    .unwrap();

                self.channel_manager = Some(channel_manager);
                self.channel_manager_tx = Some(channel_tx.clone());
            },
            _ =>()
        };


    }

    pub fn open_channel(&mut self) {
        /* fn initiate_new_channel(
            &mut self,
            route: Route,
            return_address: Address,
        ) -> OckamResult<Address> */

        match &self.channel_manager {
            Some(manager) => {
                // todo fill out
                let addr = RouterAddress::from_address(self.address.clone()).unwrap();

                let route = Route { addresses: vec![addr] };

                // todo verify
                let address = ChannelAddress(vec![0]);

                let command =
                    OckamCommand::Channel(ChannelCommand::Initiate(route, address, self.secret()));

                match &self.channel_manager_tx {
                    Some(tx) => match tx.send(command) {
                        Err(e) => log_error(e.to_string()),
                        _ => (),
                    },
                    _ => (),
                };
            }
            None => panic!("Attempted to create a Channel without a ChannelManager"),
        }
    }

    pub fn new(
        vault: Arc<Mutex<dyn XXVault + Send>>,
        address: Address,
        role: &str,
    ) -> Option<Self> {
        let node = Node::new(role).unwrap();

        let (router_tx, router_rx) = mpsc::channel::<OckamCommand>();

        let actor = Actor {
            vault,
            address,
            node,
            secret: None,
            channel_manager: None,
            channel_manager_tx: None,
            router_tx: Some(router_tx),
            router_rx: Some(router_rx)
        };
        Some(actor)
    }

    pub fn send_command(&mut self, _command: WorkerCommand) {}

    pub fn poll(&mut self) {
        self.node.poll_all().unwrap();

        match &mut self.channel_manager {
            Some(manager) => { manager.poll(); },
            _ =>()
        };

        match &self.router_rx {
            Some(router) => {
                match router.try_recv() {
                    Ok(command) => {
                        println!("{:?}",command); // TODO for now need to bridge this back in to new Node and old ChannelManager
                    },
                    _ => ()
                }
            },
            _ => ()
        }
    }

    pub fn public_key(&self) -> Option<PublicKey> {
        let key = self
            .vault
            .lock()
            .unwrap()
            .secret_public_key_get(&*self.secret().unwrap().clone());
        match key {
            Ok(public_key) => Some(public_key),
            _ => None,
        }
    }

    pub fn public_key_string(&self) -> String {
        match self.public_key() {
            Some(key) => hex::encode(key.as_ref()),
            _ => "(none)".to_string(),
        }
    }
}
