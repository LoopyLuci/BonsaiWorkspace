use dashmap::DashMap;
use std::sync::Arc;

pub type InterruptHandler = fn(u32);

pub struct InterruptManager {
    handlers: Arc<DashMap<u32, InterruptHandler>>,
}

impl InterruptManager {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(DashMap::new()),
        }
    }

    pub fn register_handler(&self, irq: u32, handler: InterruptHandler) {
        self.handlers.insert(irq, handler);
    }

    pub fn handle_interrupt(&self, irq: u32) -> bool {
        if let Some(handler) = self.handlers.get(&irq) {
            handler.value()(irq);
            true
        } else {
            false
        }
    }

    pub fn get_handler_count(&self) -> usize {
        self.handlers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_handler(_irq: u32) {}

    #[test]
    fn test_interrupt_registration() {
        let im = InterruptManager::new();
        im.register_handler(0, dummy_handler);
        assert_eq!(im.get_handler_count(), 1);
    }

    #[test]
    fn test_interrupt_handling() {
        let im = InterruptManager::new();
        im.register_handler(0, dummy_handler);
        assert!(im.handle_interrupt(0));
    }
}
