use crate::KernelError;
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::sync::Arc;
use async_trait::async_trait;
use tracing::{info, error};

pub type InterruptNumber = u32;

#[async_trait]
pub trait InterruptHandlerTrait: Send + Sync {
    async fn handle(&self, number: InterruptNumber) -> Result<(), KernelError>;
}

/// Interrupt descriptor
pub struct InterruptDescriptor {
    pub number: InterruptNumber,
    pub handler: Arc<dyn InterruptHandlerTrait>,
    pub enabled: RwLock<bool>,
}

/// Interrupt controller
pub struct InterruptController {
    handlers: RwLock<BTreeMap<InterruptNumber, Arc<InterruptDescriptor>>>,
}

impl InterruptController {
    pub fn new() -> Self {
        InterruptController {
            handlers: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn register_handler(
        &self,
        number: InterruptNumber,
        handler: Arc<dyn InterruptHandlerTrait>,
    ) -> Result<(), KernelError> {
        let mut handlers = self.handlers.write();

        if handlers.contains_key(&number) {
            return Err(KernelError::InterruptError(
                "Interrupt already registered".to_string(),
            ));
        }

        handlers.insert(
            number,
            Arc::new(InterruptDescriptor {
                number,
                handler,
                enabled: RwLock::new(true),
            }),
        );

        info!("Registered interrupt handler for {}", number);

        Ok(())
    }

    pub fn get_handler(&self, number: InterruptNumber) -> Option<Arc<InterruptDescriptor>> {
        self.handlers.read().get(&number).cloned()
    }

    pub async fn dispatch(&self, number: InterruptNumber) -> Result<(), KernelError> {
        if let Some(descriptor) = self.get_handler(number) {
            if *descriptor.enabled.read() {
                descriptor.handler.handle(number).await?;
            }
        } else {
            error!("No handler for interrupt {}", number);
        }

        Ok(())
    }

    pub fn enable_interrupt(&self, number: InterruptNumber) -> Result<(), KernelError> {
        if let Some(descriptor) = self.get_handler(number) {
            *descriptor.enabled.write() = true;
            Ok(())
        } else {
            Err(KernelError::InterruptError(
                "Interrupt not registered".to_string(),
            ))
        }
    }

    pub fn disable_interrupt(&self, number: InterruptNumber) -> Result<(), KernelError> {
        if let Some(descriptor) = self.get_handler(number) {
            *descriptor.enabled.write() = false;
            Ok(())
        } else {
            Err(KernelError::InterruptError(
                "Interrupt not registered".to_string(),
            ))
        }
    }
}

/// Main interrupt handler
pub struct InterruptHandler {
    controller: Arc<InterruptController>,
}

impl InterruptHandler {
    pub fn new() -> Result<Self, KernelError> {
        info!("Initializing interrupt handler");

        Ok(InterruptHandler {
            controller: Arc::new(InterruptController::new()),
        })
    }

    pub fn controller(&self) -> Arc<InterruptController> {
        Arc::clone(&self.controller)
    }

    pub async fn run(&self) -> Result<(), KernelError> {
        info!("Starting interrupt handler main loop");

        loop {
            // In a real implementation, wait for interrupts from hardware
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    pub async fn handle_interrupt(&self, number: InterruptNumber) -> Result<(), KernelError> {
        self.controller.dispatch(number).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler;

    #[async_trait]
    impl crate::interrupt::InterruptHandlerTrait for TestHandler {
        async fn handle(&self, _number: InterruptNumber) -> Result<(), KernelError> {
            Ok(())
        }
    }

    #[test]
    fn test_interrupt_handler_creation() {
        let ih = InterruptHandler::new();
        assert!(ih.is_ok());
    }

    #[tokio::test]
    async fn test_interrupt_registration() {
        let ih = InterruptHandler::new().unwrap();
        let controller = ih.controller();

        let handler = Arc::new(TestHandler);
        let result = controller.register_handler(0, handler);

        assert!(result.is_ok());
    }
}
