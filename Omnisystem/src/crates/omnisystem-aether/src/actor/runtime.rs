//! Actor Runtime — Message-Driven Concurrency
//!
//! Base actor infrastructure with supervision and location transparency.


/// Base trait for actors
pub trait Actor: Send + Sync {
    fn on_start(&mut self);
    fn on_stop(&mut self);
}

/// Reference to a running actor
#[derive(Clone)]
pub struct ActorRef {
    id: uuid::Uuid,
}

impl ActorRef {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
        }
    }

    pub fn id(&self) -> uuid::Uuid {
        self.id
    }
}

/// The actor runtime manages actor lifecycle and message delivery
pub struct ActorRuntime {
    actors: std::collections::HashMap<uuid::Uuid, Box<dyn Actor>>,
}

impl ActorRuntime {
    pub fn new() -> Self {
        Self {
            actors: std::collections::HashMap::new(),
        }
    }

    pub fn spawn<A: Actor + 'static>(&mut self, actor: A) -> ActorRef {
        let aref = ActorRef::new();
        self.actors.insert(aref.id(), Box::new(actor));
        aref
    }
}
