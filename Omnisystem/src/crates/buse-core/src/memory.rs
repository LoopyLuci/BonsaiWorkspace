use dashmap::DashMap;
use std::sync::Arc;

pub struct MemoryBus {
    ram: Vec<u8>,
    mmio: DashMap<u64, Arc<dyn MmioDevice>>,
}

impl MemoryBus {
    pub fn new(ram_size_bytes: u64) -> Self {
        Self {
            ram: vec![0; ram_size_bytes as usize],
            mmio: DashMap::new(),
        }
    }

    pub fn with_ram(size: u64) -> Self {
        Self::new(size)
    }

    pub fn read_u8(&self, addr: u64) -> u8 {
        if let Some(dev) = self.mmio.get(&addr) {
            return dev.read_u8(addr);
        }
        self.ram.get(addr as usize).copied().unwrap_or(0)
    }

    pub fn write_u8(&mut self, addr: u64, value: u8) {
        if let Some(dev) = self.mmio.get(&addr) {
            dev.write_u8(addr, value);
            return;
        }
        if (addr as usize) < self.ram.len() {
            self.ram[addr as usize] = value;
        }
    }

    pub fn read_u32(&self, addr: u64) -> u32 {
        u32::from_le_bytes([
            self.read_u8(addr),
            self.read_u8(addr + 1),
            self.read_u8(addr + 2),
            self.read_u8(addr + 3),
        ])
    }

    pub fn write_u32(&mut self, addr: u64, value: u32) {
        let bytes = value.to_le_bytes();
        self.write_u8(addr, bytes[0]);
        self.write_u8(addr + 1, bytes[1]);
        self.write_u8(addr + 2, bytes[2]);
        self.write_u8(addr + 3, bytes[3]);
    }

    pub fn clear(&mut self) {
        self.ram.fill(0);
    }

    pub fn map_mmio(&self, addr: u64, device: Arc<dyn MmioDevice>) {
        self.mmio.insert(addr, device);
    }

    pub fn len(&self) -> usize {
        self.ram.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ram.is_empty()
    }
}

pub trait MmioDevice: Send + Sync {
    fn read_u8(&self, addr: u64) -> u8;
    fn write_u8(&self, addr: u64, value: u8);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU8, Ordering};

    #[test]
    fn test_ram_read_write_u8() {
        let mut bus = MemoryBus::new(16);
        bus.write_u8(4, 0xAB);
        assert_eq!(bus.read_u8(4), 0xAB);
        assert_eq!(bus.read_u8(5), 0);
    }

    #[test]
    fn test_u32_roundtrip_is_little_endian() {
        let mut bus = MemoryBus::new(16);
        bus.write_u32(0, 0x11223344);
        assert_eq!(bus.read_u8(0), 0x44);
        assert_eq!(bus.read_u8(1), 0x33);
        assert_eq!(bus.read_u8(2), 0x22);
        assert_eq!(bus.read_u8(3), 0x11);
        assert_eq!(bus.read_u32(0), 0x11223344);
    }

    #[test]
    fn test_out_of_bounds_read_is_zero_and_write_is_noop() {
        let mut bus = MemoryBus::new(4);
        assert_eq!(bus.read_u8(1000), 0);
        bus.write_u8(1000, 0xFF); // must not panic
        assert_eq!(bus.read_u8(1000), 0);
    }

    #[test]
    fn test_clear_zeroes_ram() {
        let mut bus = MemoryBus::new(8);
        bus.write_u32(0, 0xFFFFFFFF);
        bus.clear();
        assert_eq!(bus.read_u32(0), 0);
    }

    struct CountingDevice {
        reads: AtomicU8,
        last_write: AtomicU8,
    }

    impl MmioDevice for CountingDevice {
        fn read_u8(&self, _addr: u64) -> u8 {
            self.reads.fetch_add(1, Ordering::SeqCst);
            0x42
        }
        fn write_u8(&self, _addr: u64, value: u8) {
            self.last_write.store(value, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_mmio_device_intercepts_ram_access() {
        let mut bus = MemoryBus::new(16);
        let device = Arc::new(CountingDevice {
            reads: AtomicU8::new(0),
            last_write: AtomicU8::new(0),
        });
        bus.map_mmio(8, device.clone());

        assert_eq!(bus.read_u8(8), 0x42);
        assert_eq!(device.reads.load(Ordering::SeqCst), 1);

        bus.write_u8(8, 0x99);
        assert_eq!(device.last_write.load(Ordering::SeqCst), 0x99);

        // Untouched addresses still hit real RAM, not the device.
        assert_eq!(bus.read_u8(9), 0);
    }
}
