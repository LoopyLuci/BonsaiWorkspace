//! Interrupt controller and delivery system

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Interrupt types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InterruptType {
    /// Timer interrupt
    Timer,
    /// External interrupt
    External,
    /// I/O interrupt
    IoInterrupt,
    /// Page fault
    PageFault,
    /// General protection fault
    GeneralProtectionFault,
    /// Divide by zero
    DivideByZero,
    /// Illegal instruction
    IllegalInstruction,
    /// NMI (Non-Maskable Interrupt)
    Nmi,
    /// Custom interrupt with ID
    Custom(u16),
}

impl std::fmt::Display for InterruptType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timer => write!(f, "Timer"),
            Self::External => write!(f, "External"),
            Self::IoInterrupt => write!(f, "I/O"),
            Self::PageFault => write!(f, "PageFault"),
            Self::GeneralProtectionFault => write!(f, "GPF"),
            Self::DivideByZero => write!(f, "DivideByZero"),
            Self::IllegalInstruction => write!(f, "IllegalInstruction"),
            Self::Nmi => write!(f, "NMI"),
            Self::Custom(id) => write!(f, "Custom({})", id),
        }
    }
}

/// Interrupt descriptor: metadata about an interrupt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptDescriptor {
    /// Interrupt type
    pub interrupt_type: InterruptType,
    /// Priority level (higher = more urgent)
    pub priority: u8,
    /// Whether this interrupt can be masked
    pub maskable: bool,
}

impl InterruptDescriptor {
    /// Create a new interrupt descriptor
    pub fn new(interrupt_type: InterruptType, priority: u8, maskable: bool) -> Self {
        Self {
            interrupt_type,
            priority,
            maskable,
        }
    }
}

/// Interrupt controller manages pending interrupts for all cores
#[derive(Debug)]
pub struct InterruptController {
    /// Pending interrupts per core
    pending: HashMap<usize, Vec<InterruptType>>,
    /// Interrupt descriptors
    descriptors: HashMap<InterruptType, InterruptDescriptor>,
    /// Interrupt masks per core
    masks: HashMap<usize, u64>,
    /// Total number of cores
    num_cores: usize,
}

impl InterruptController {
    /// Create a new interrupt controller for the given number of cores
    pub fn new(num_cores: usize) -> Self {
        let mut descriptors = HashMap::new();

        // Register standard interrupt descriptors
        descriptors.insert(
            InterruptType::Timer,
            InterruptDescriptor::new(InterruptType::Timer, 32, true),
        );
        descriptors.insert(
            InterruptType::External,
            InterruptDescriptor::new(InterruptType::External, 33, true),
        );
        descriptors.insert(
            InterruptType::IoInterrupt,
            InterruptDescriptor::new(InterruptType::IoInterrupt, 34, true),
        );
        descriptors.insert(
            InterruptType::PageFault,
            InterruptDescriptor::new(InterruptType::PageFault, 14, false),
        );
        descriptors.insert(
            InterruptType::GeneralProtectionFault,
            InterruptDescriptor::new(InterruptType::GeneralProtectionFault, 13, false),
        );
        descriptors.insert(
            InterruptType::DivideByZero,
            InterruptDescriptor::new(InterruptType::DivideByZero, 0, false),
        );
        descriptors.insert(
            InterruptType::IllegalInstruction,
            InterruptDescriptor::new(InterruptType::IllegalInstruction, 6, false),
        );
        descriptors.insert(
            InterruptType::Nmi,
            InterruptDescriptor::new(InterruptType::Nmi, 255, false),
        );

        let mut pending = HashMap::new();
        let mut masks = HashMap::new();

        for i in 0..num_cores {
            pending.insert(i, Vec::new());
            masks.insert(i, 0); // All interrupts enabled initially
        }

        Self {
            pending,
            descriptors,
            masks,
            num_cores,
        }
    }

    /// Queue an interrupt for a specific core
    pub fn queue_interrupt(&mut self, core_id: usize, interrupt_type: InterruptType) {
        if core_id < self.num_cores {
            if let Some(pending) = self.pending.get_mut(&core_id) {
                pending.push(interrupt_type);
            }
        }
    }

    /// Dequeue the next pending interrupt for a core
    pub fn dequeue_interrupt(&mut self, core_id: usize) -> Option<InterruptType> {
        if core_id < self.num_cores {
            if let Some(pending) = self.pending.get_mut(&core_id) {
                if !pending.is_empty() {
                    return Some(pending.remove(0));
                }
            }
        }
        None
    }

    /// Check if a core has pending interrupts
    pub fn has_pending(&self, core_id: usize) -> bool {
        if core_id < self.num_cores {
            if let Some(pending) = self.pending.get(&core_id) {
                return !pending.is_empty();
            }
        }
        false
    }

    /// Get all pending interrupts for a core without removing them
    pub fn pending_interrupts(&self, core_id: usize) -> Vec<InterruptType> {
        if core_id < self.num_cores {
            if let Some(pending) = self.pending.get(&core_id) {
                return pending.clone();
            }
        }
        Vec::new()
    }

    /// Set interrupt mask for a core (bit per interrupt type)
    pub fn set_mask(&mut self, core_id: usize, mask: u64) {
        if core_id < self.num_cores {
            self.masks.insert(core_id, mask);
        }
    }

    /// Get interrupt mask for a core
    pub fn get_mask(&self, core_id: usize) -> u64 {
        self.masks.get(&core_id).copied().unwrap_or(0)
    }

    /// Reset the interrupt controller
    pub fn reset(&mut self) {
        for (_, pending) in self.pending.iter_mut() {
            pending.clear();
        }
        for (_, mask) in self.masks.iter_mut() {
            *mask = 0; // All interrupts enabled
        }
    }

    /// Register a custom interrupt descriptor
    pub fn register_interrupt(&mut self, interrupt_type: InterruptType, descriptor: InterruptDescriptor) {
        self.descriptors.insert(interrupt_type, descriptor);
    }

    /// Get interrupt descriptor
    pub fn get_descriptor(&self, interrupt_type: InterruptType) -> Option<&InterruptDescriptor> {
        self.descriptors.get(&interrupt_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_controller_creation() {
        let controller = InterruptController::new(8);
        assert!(!controller.has_pending(0));
        assert!(!controller.has_pending(7));
    }

    #[test]
    fn test_queue_and_dequeue() {
        let mut controller = InterruptController::new(4);
        controller.queue_interrupt(0, InterruptType::Timer);
        controller.queue_interrupt(0, InterruptType::External);

        assert!(controller.has_pending(0));
        assert_eq!(controller.dequeue_interrupt(0), Some(InterruptType::Timer));
        assert!(controller.has_pending(0));
        assert_eq!(controller.dequeue_interrupt(0), Some(InterruptType::External));
        assert!(!controller.has_pending(0));
    }

    #[test]
    fn test_pending_interrupts() {
        let mut controller = InterruptController::new(4);
        controller.queue_interrupt(1, InterruptType::Timer);
        controller.queue_interrupt(1, InterruptType::PageFault);

        let pending = controller.pending_interrupts(1);
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0], InterruptType::Timer);
        assert_eq!(pending[1], InterruptType::PageFault);
    }

    #[test]
    fn test_interrupt_masks() {
        let mut controller = InterruptController::new(4);
        assert_eq!(controller.get_mask(0), 0);

        controller.set_mask(0, 0xFF);
        assert_eq!(controller.get_mask(0), 0xFF);

        controller.set_mask(0, 0x00);
        assert_eq!(controller.get_mask(0), 0x00);
    }

    #[test]
    fn test_interrupt_descriptor_registration() {
        let mut controller = InterruptController::new(4);
        let custom = InterruptType::Custom(100);
        let descriptor = InterruptDescriptor::new(custom, 50, true);

        controller.register_interrupt(custom, descriptor);
        let retrieved = controller.get_descriptor(custom);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().priority, 50);
    }

    #[test]
    fn test_interrupt_controller_reset() {
        let mut controller = InterruptController::new(4);
        controller.queue_interrupt(0, InterruptType::Timer);
        controller.queue_interrupt(1, InterruptType::External);
        controller.set_mask(0, 0xFF);

        controller.reset();
        assert!(!controller.has_pending(0));
        assert!(!controller.has_pending(1));
        assert_eq!(controller.get_mask(0), 0);
    }

    #[test]
    fn test_interrupt_type_display() {
        assert_eq!(InterruptType::Timer.to_string(), "Timer");
        assert_eq!(InterruptType::Nmi.to_string(), "NMI");
        assert_eq!(InterruptType::Custom(42).to_string(), "Custom(42)");
    }
}
