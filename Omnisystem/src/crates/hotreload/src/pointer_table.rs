use dashmap::DashMap;

/// Wrapper for raw pointers to make them Send + Sync (safe for raw address sharing)
#[repr(transparent)]
struct SendSyncPtr(*const ());

unsafe impl Send for SendSyncPtr {}
unsafe impl Sync for SendSyncPtr {}

pub struct FunctionPointerTable {
    table: DashMap<String, SendSyncPtr>,
}

impl FunctionPointerTable {
    pub fn new() -> Self {
        Self {
            table: DashMap::new(),
        }
    }

    pub fn set(&self, name: &str, ptr: *const ()) {
        self.table.insert(name.to_string(), SendSyncPtr(ptr));
    }

    pub fn get(&self, name: &str) -> Option<*const ()> {
        self.table.get(name).map(|entry| entry.0)
    }

    pub fn swap(&self, name: &str, new_ptr: *const ()) -> Result<*const (), anyhow::Error> {
        let old = self.table.insert(name.to_string(), SendSyncPtr(new_ptr));
        Ok(old.map(|old_ptr| old_ptr.0).unwrap_or(std::ptr::null()))
    }

    pub fn contains(&self, name: &str) -> bool {
        self.table.contains_key(name)
    }
}

impl Default for FunctionPointerTable {
    fn default() -> Self {
        Self::new()
    }
}
