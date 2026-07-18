use super::{FunctionPointerTable, AtomicTransaction};
use std::collections::HashMap;
use anyhow::Result;

pub struct HotReloadRuntime {
    tables: HashMap<String, FunctionPointerTable>,
    transaction: AtomicTransaction,
}

impl HotReloadRuntime {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            transaction: AtomicTransaction::new(),
        }
    }

    pub fn create_table(&mut self, name: &str) -> &FunctionPointerTable {
        self.tables.entry(name.to_string()).or_insert_with(FunctionPointerTable::new);
        &self.tables[name]
    }

    pub fn get_table(&self, name: &str) -> Option<&FunctionPointerTable> {
        self.tables.get(name)
    }

    pub fn register_function(&mut self, name: &str, ptr: *const ()) {
        // Default to "default" table if no namespace is specified
        let table = self.create_table("default");
        table.set(name, ptr);
    }

    pub fn get_function(&self, name: &str) -> Option<*const ()> {
        self.tables.get("default").and_then(|table| table.get(name))
    }

    pub fn replace_function(&self, module: &str, func: &str, new_ptr: *const ()) -> Result<*const ()> {
        let table = self.tables.get(module).ok_or_else(|| anyhow::anyhow!("Module not found"))?;
        table.swap(func, new_ptr)
    }

    pub fn begin_transaction(&mut self) -> &mut AtomicTransaction {
        self.transaction = AtomicTransaction::new();
        &mut self.transaction
    }

    pub fn commit_transaction(&mut self) -> Result<()> {
        self.transaction.commit()
    }

    pub fn rollback_transaction(&self) {
        self.transaction.rollback();
    }
}

impl Default for HotReloadRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    extern "C" fn func_a() -> i32 {
        1
    }

    extern "C" fn func_b() -> i32 {
        2
    }

    #[test]
    fn test_create_and_get_table() {
        let mut runtime = HotReloadRuntime::new();
        runtime.create_table("mod1");
        assert!(runtime.get_table("mod1").is_some());
        assert!(runtime.get_table("mod2").is_none());
    }

    #[test]
    fn test_register_and_get_function_default_table() {
        let mut runtime = HotReloadRuntime::new();
        let ptr = func_a as *const ();
        runtime.register_function("greet", ptr);

        assert_eq!(runtime.get_function("greet"), Some(ptr));
        assert!(runtime.get_table("default").is_some());
    }

    #[test]
    fn test_replace_function_returns_old_pointer() {
        let mut runtime = HotReloadRuntime::new();
        runtime.create_table("mod1");
        let old_ptr = func_a as *const ();
        runtime.get_table("mod1").unwrap().set("handler", old_ptr);

        let new_ptr = func_b as *const ();
        let replaced = runtime.replace_function("mod1", "handler", new_ptr).unwrap();

        assert_eq!(replaced, old_ptr);
        assert_eq!(runtime.get_table("mod1").unwrap().get("handler"), Some(new_ptr));
    }

    #[test]
    fn test_replace_function_unknown_module_errors() {
        let runtime = HotReloadRuntime::new();
        let result = runtime.replace_function("nope", "handler", func_a as *const ());
        assert!(result.is_err());
    }

    #[test]
    fn test_transaction_lifecycle() {
        let mut runtime = HotReloadRuntime::new();
        {
            let tx = runtime.begin_transaction();
            assert!(!tx.is_committed());
        }
        runtime.commit_transaction().unwrap();
    }

    #[test]
    fn test_rollback_transaction_restores_snapshot() {
        let mut runtime = HotReloadRuntime::new();
        let state = Arc::new(parking_lot::RwLock::new(1u32));
        let saved = *state.read();

        {
            let tx = runtime.begin_transaction();
            tx.add_snapshot(crate::StateSnapshot::new(state.clone(), saved));
        }

        *state.write() = 42;
        runtime.rollback_transaction();
        assert_eq!(*state.read(), 1);
    }
}
