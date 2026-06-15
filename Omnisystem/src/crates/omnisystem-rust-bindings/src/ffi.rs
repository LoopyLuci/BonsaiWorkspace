/// FFI bindings - Polyglot communication layer

use std::sync::Arc;
use omnisystem_ffi::FFIRegistry;

pub struct FFIBridge {
    registry: Arc<FFIRegistry>,
}

impl FFIBridge {
    pub fn new(registry: Arc<FFIRegistry>) -> Self {
        FFIBridge { registry }
    }

    pub fn register_module(&self, name: &str, version: (u32, u32, u32)) {
        let module = Arc::new(omnisystem_ffi::FFIModule::new(name, version));
        self.registry.register_module(module);
    }

    pub fn list_modules(&self) -> Vec<String> {
        self.registry.list_modules()
    }

    pub fn module_count(&self) -> usize {
        self.registry.module_count()
    }
}
