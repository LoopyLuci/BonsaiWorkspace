use crate::Result;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ResourceMetrics {
    pub used: usize,
    pub available: usize,
    pub total: usize,
}

pub struct ResourcePool {
    total: usize,
    used: Arc<Mutex<usize>>,
}

impl ResourcePool {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            used: Arc::new(Mutex::new(0)),
        }
    }

    pub fn allocate(&self, amount: usize) -> Result<()> {
        let mut used = self.used.lock();
        if *used + amount > self.total {
            return Err(crate::RuntimeError::ResourceExhausted(
                format!("Need {} but only {} available", amount, self.total - *used)
            ));
        }
        *used += amount;
        Ok(())
    }

    pub fn release(&self, amount: usize) {
        let mut used = self.used.lock();
        *used = used.saturating_sub(amount);
    }

    pub fn metrics(&self) -> ResourceMetrics {
        let used = *self.used.lock();
        ResourceMetrics {
            used,
            available: self.total - used,
            total: self.total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_allocation() {
        let pool = ResourcePool::new(100);
        assert!(pool.allocate(50).is_ok());
        assert_eq!(pool.metrics().used, 50);
    }

    #[test]
    fn test_pool_exhaustion() {
        let pool = ResourcePool::new(100);
        pool.allocate(100).unwrap();
        assert!(pool.allocate(1).is_err());
    }
}
