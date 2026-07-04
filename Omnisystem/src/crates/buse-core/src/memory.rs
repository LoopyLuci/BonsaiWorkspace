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
