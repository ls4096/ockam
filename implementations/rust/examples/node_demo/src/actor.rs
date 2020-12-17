use std::net::SocketAddr;
use std::str;
use std::sync::{Arc, Mutex};

use ockam::kex::CipherSuite;
use ockam::message::Address;
use ockam::system::commands::WorkerCommand;
use ockam::vault::types::{
    PublicKey, SecretAttributes, SecretPersistence, SecretType, CURVE25519_SECRET_LENGTH,
};
use ockam::vault::Secret;
use ockam_kex_xx::{XXNewKeyExchanger, XXVault};
use ockam_node::Node;

pub struct Actor {
    vault: Arc<Mutex<dyn XXVault + Send>>,
    address: Address,
    secret: Option<Arc<Box<dyn Secret>>>,
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

    pub fn secret(&self) -> Option<&Box<dyn Secret>> {
        match &self.secret {
            Some(s) => Some(&s),
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
        self.node.initialize_transport(listen_address);
    }

    pub fn connect(&mut self, _address_str: &str) -> Option<Address> {
        None
    }

    pub fn default_new_key_exchanger(&self) -> XXNewKeyExchanger {
        XXNewKeyExchanger::new(
            CipherSuite::Curve25519AesGcmSha256,
            self.vault.clone(),
            self.vault.clone(),
        )
    }

    pub fn new(vault: Arc<Mutex<dyn XXVault + Send>>, address: Address) -> Option<Self> {
        let node = Node::new("alice").unwrap();

        let actor = Actor {
            vault,
            address,
            node,
            secret: None,
        };
        Some(actor)
    }

    pub fn send_command(&mut self, _command: WorkerCommand) {}

    pub fn poll(&mut self) {
        self.node.poll_all().unwrap()
    }

    pub fn open_channel(&mut self, _address: Address) {

    }

    pub fn public_key(&self) -> Option<PublicKey> {
        let key = self
            .vault
            .lock()
            .unwrap()
            .secret_public_key_get(self.secret().unwrap());
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
