/// FFI Callbacks - Function pointers and async callbacks

use std::sync::Arc;
use parking_lot::RwLock;

pub type CallbackFn = extern "C" fn(*mut std::ffi::c_void) -> i32;
pub type AsyncCallbackFn = Box<dyn Fn() + Send + Sync>;

/// Callback handle
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallbackHandle(u64);

impl CallbackHandle {
    pub fn new(id: u64) -> Self {
        CallbackHandle(id)
    }

    pub fn id(&self) -> u64 {
        self.0
    }
}

/// Callback registration
pub struct Callback {
    pub handle: CallbackHandle,
    pub function: CallbackFn,
    pub user_data: *mut std::ffi::c_void,
}

/// Callback manager
pub struct CallbackManager {
    callbacks: RwLock<std::collections::HashMap<CallbackHandle, Callback>>,
    next_handle: RwLock<u64>,
}

impl CallbackManager {
    pub fn new() -> Self {
        CallbackManager {
            callbacks: RwLock::new(std::collections::HashMap::new()),
            next_handle: RwLock::new(1),
        }
    }

    pub fn register(
        &self,
        function: CallbackFn,
        user_data: *mut std::ffi::c_void,
    ) -> CallbackHandle {
        let handle = {
            let mut next = self.next_handle.write();
            let h = CallbackHandle(*next);
            *next += 1;
            h
        };

        let callback = Callback {
            handle,
            function,
            user_data,
        };

        self.callbacks.write().insert(handle, callback);
        handle
    }

    pub fn call(&self, handle: CallbackHandle) -> Result<i32, String> {
        let callbacks = self.callbacks.read();

        match callbacks.get(&handle) {
            Some(cb) => {
                let result = (cb.function)(cb.user_data);
                Ok(result)
            }
            None => Err("Callback not found".to_string()),
        }
    }

    pub fn unregister(&self, handle: CallbackHandle) -> bool {
        self.callbacks.write().remove(&handle).is_some()
    }

    pub fn callback_count(&self) -> usize {
        self.callbacks.read().len()
    }
}

/// Async callback support
pub struct AsyncCallback {
    pub handle: CallbackHandle,
    pub callback: Arc<AsyncCallbackFn>,
}

pub struct AsyncCallbackManager {
    callbacks: RwLock<std::collections::HashMap<CallbackHandle, AsyncCallback>>,
    next_handle: RwLock<u64>,
}

impl AsyncCallbackManager {
    pub fn new() -> Self {
        AsyncCallbackManager {
            callbacks: RwLock::new(std::collections::HashMap::new()),
            next_handle: RwLock::new(1),
        }
    }

    pub fn register<F>(&self, callback: F) -> CallbackHandle
    where
        F: Fn() + Send + Sync + 'static,
    {
        let handle = {
            let mut next = self.next_handle.write();
            let h = CallbackHandle(*next);
            *next += 1;
            h
        };

        let async_cb = AsyncCallback {
            handle,
            callback: Arc::new(Box::new(callback)),
        };

        self.callbacks.write().insert(handle, async_cb);
        handle
    }

    pub async fn call_async(&self, handle: CallbackHandle) -> Result<(), String> {
        let callbacks = self.callbacks.read();

        match callbacks.get(&handle) {
            Some(cb) => {
                (cb.callback)();
                Ok(())
            }
            None => Err("Async callback not found".to_string()),
        }
    }

    pub fn unregister(&self, handle: CallbackHandle) -> bool {
        self.callbacks.write().remove(&handle).is_some()
    }

    pub fn callback_count(&self) -> usize {
        self.callbacks.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn test_callback(_user_data: *mut std::ffi::c_void) -> i32 {
        42
    }

    #[test]
    fn test_callback_manager() {
        let manager = CallbackManager::new();

        let handle = manager.register(test_callback, std::ptr::null_mut());
        assert!(manager.callback_count() > 0);

        let result = manager.call(handle);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_unregister() {
        let manager = CallbackManager::new();
        let handle = manager.register(test_callback, std::ptr::null_mut());

        assert!(manager.unregister(handle));
        assert!(manager.call(handle).is_err());
    }

    #[test]
    fn test_async_callback_manager() {
        let manager = AsyncCallbackManager::new();

        let counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let counter_clone = counter.clone();

        let handle = manager.register(move || {
            counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });

        assert!(manager.callback_count() > 0);
        assert!(manager.unregister(handle));
    }
}
