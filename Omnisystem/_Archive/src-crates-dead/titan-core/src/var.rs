//! Var - dynamic scoped variable binding

use std::cell::RefCell;

/// Dynamic variable with lexical scoping
pub struct Var<T: Clone> {
    value: RefCell<T>,
}

impl<T: Clone> Var<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: RefCell::new(value),
        }
    }

    pub fn deref(&self) -> T {
        self.value.borrow().clone()
    }

    pub fn bind<F, R>(&self, new_value: T, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let old_value = self.deref();
        *self.value.borrow_mut() = new_value;
        let result = f();
        *self.value.borrow_mut() = old_value;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_binding() {
        let var = Var::new(5);
        assert_eq!(var.deref(), 5);

        let result = var.bind(10, || {
            assert_eq!(var.deref(), 10);
            var.deref() * 2
        });

        assert_eq!(result, 20);
        assert_eq!(var.deref(), 5); // Restored after binding
    }
}
