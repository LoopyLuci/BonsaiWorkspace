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
