use std::marker::PhantomData;

#[repr(align(64))]  // Cache line size: 64 bytes
pub struct CacheAlignedLayout<T> {
    value: T,
    _phantom: PhantomData<T>,
}

impl<T> CacheAlignedLayout<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            _phantom: PhantomData,
        }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }
}
