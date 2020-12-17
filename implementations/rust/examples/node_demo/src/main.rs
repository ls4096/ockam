use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use hashbrown::HashMap;
use rand::{RngCore, thread_rng};
use rand::prelude::ThreadRng;

use ockam::message::Address;
use ockam_kex_xx::XXVault;
use ockam_vault_software::DefaultVault;

use crate::actor::{Actor, Addressable};
use crate::log::{log_entries, log_error};

pub mod actor;
pub mod log;

struct App {
    rng: ThreadRng,
    vault: Arc<Mutex<dyn XXVault + Send>>,
    actors: HashMap<String, Rc<RefCell<Actor>>>,
}

trait WorkerAddress {
    fn worker_address(&self) -> Result<Address, String>;
}

impl WorkerAddress for &str {
    fn worker_address(&self) -> Result<Address, String> {
        Address::worker_address_from_string(self)
    }
}

type ActorHandle = Rc<RefCell<Actor>>;

impl App {
    fn create() -> App {
        let v = DefaultVault::default();
        let vault = Arc::new(Mutex::new(v));

        let mut bench = App {
            rng: thread_rng(),
            vault,
            actors: HashMap::new(),
        };

        bench.setup_scenario();
        bench
    }

    fn create_anonymous_actor(&mut self) -> Option<&ActorHandle> {
        let id = hex::encode(self.rng.next_u32().to_le_bytes());
        self.create_actor(&id, "?")
    }

    fn create_client(&mut self) -> Option<&ActorHandle> {
        match self.create_anonymous_actor() {
            Some(server) => {
                let mut actor = server.borrow_mut();
                actor.create_client_transport();
                actor.create_channel_manager();
                Some(server)
            }
            _ => None,
        }
    }

    fn create_actor(&mut self, address: &str, role: &str) -> Option<&ActorHandle> {
        match Actor::new(self.vault.clone(), address.worker_address().unwrap(), role) {
            Some(mut actor) => {
                self.generate_secret(&mut actor);
                let actor_address = actor.address().as_string();
                self.actors
                    .insert(actor_address.clone(), Rc::new(RefCell::new(actor)));
                self.actors.get(&actor_address.clone())
            }
            None => panic!("can't create Actor"),
        }
    }

    fn generate_secret(&mut self, actor: &mut Actor) {
        let attribs = actor.secret_attributes();
        match self.vault.lock().unwrap().secret_generate(attribs) {
            Ok(secret) => actor.set_secret(secret),
            _ => log_error("generate_secret_fail"),
        };
    }

    fn poll(&mut self) {
        for actor in self.actors.values_mut() {
            actor.borrow_mut().poll();
        }
    }

    fn create_server(&mut self, socket_address: &str) -> Option<&ActorHandle> {
        match self.create_anonymous_actor() {
            Some(server) => {
                let mut actor = server.borrow_mut();
                actor.create_server_transport(socket_address);
                actor.create_channel_manager();
                Some(server)
            }
            _ => None,
        }
    }

    fn setup_scenario(&mut self) {
        let mut create_client = || -> (String, String) {
            let client_ref = self.create_client().unwrap();
            let client = client_ref.borrow();
            (client.address().as_string(), client.public_key_string())
        };

        let (alice_address, alice_pubkey) = create_client();

        println!(
            "Alice: {{ address: {}, pubkey: {} }}",
            alice_address, alice_pubkey
        );

        let mut create_server = |address: &str| -> (String, String) {
            let server_ref = self.create_server(address).unwrap();
            let server = server_ref.borrow();
            (server.address().as_string(), server.public_key_string())
        };

        let (bob_address, bob_pubkey) = create_server("127.0.0.1:9999");

        println!(
            "Bob: {{ address: {}, pubkey: {} }}",
            bob_address, bob_pubkey
        );
    }
}

fn main() {
    let mut app = App::create();

    loop {
        app.poll();
        let _log = log_entries();
        thread::sleep(Duration::from_millis(500));
    }
}
