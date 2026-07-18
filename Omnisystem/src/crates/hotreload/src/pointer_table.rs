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

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn func_a() -> i32 {
        1
    }

    extern "C" fn func_b() -> i32 {
        2
    }

    #[test]
    fn test_set_and_get() {
        let table = FunctionPointerTable::new();
        let ptr = func_a as *const ();
        table.set("handler", ptr);

        assert!(table.contains("handler"));
        assert_eq!(table.get("handler"), Some(ptr));
        assert_eq!(table.get("missing"), None);
    }

    #[test]
    fn test_swap_returns_old_pointer() {
        let table = FunctionPointerTable::new();
        let old_ptr = func_a as *const ();
        let new_ptr = func_b as *const ();

        table.set("handler", old_ptr);
        let returned_old = table.swap("handler", new_ptr).unwrap();

        assert_eq!(returned_old, old_ptr);
        assert_eq!(table.get("handler"), Some(new_ptr));
    }

    #[test]
    fn test_swap_on_empty_slot_returns_null() {
        let table = FunctionPointerTable::new();
        let new_ptr = func_b as *const ();
        let returned_old = table.swap("fresh", new_ptr).unwrap();

        assert!(returned_old.is_null());
        assert_eq!(table.get("fresh"), Some(new_ptr));
    }
}
