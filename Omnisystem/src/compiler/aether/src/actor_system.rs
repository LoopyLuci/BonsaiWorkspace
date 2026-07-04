// AETHER ACTOR SYSTEM - Distributed Computing Model

use std::collections::HashMap;

pub struct Actor {
    pub id: String,
    pub mailbox: Vec<Message>,
}

pub struct Message {
    pub sender: String,
    pub content: String,
}

pub struct ActorSystem {
    pub actors: HashMap<String, Actor>,
    pub replicas: usize,
}

impl ActorSystem {
    pub fn new() -> Self {
        ActorSystem {
            actors: HashMap::new(),
            replicas: 3,
        }
    }

    pub fn spawn_actor(&mut self, id: String) {
        self.actors.insert(id, Actor {
            id: String::new(),
            mailbox: Vec::new(),
        });
    }

    pub fn send_message(&mut self, from: String, to: String, content: String) {
        if let Some(actor) = self.actors.get_mut(&to) {
            actor.mailbox.push(Message { sender: from, content });
        }
    }

    pub fn replicate(&self) {
        println!("Replicating actors to {} regions", self.replicas);
    }
}
