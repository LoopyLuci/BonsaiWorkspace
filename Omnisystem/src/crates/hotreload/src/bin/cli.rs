//! CLI for exercising the hotreload crate: registers a function pointer,
//! hot-swaps it, and demonstrates transactional rollback of shared state.

use hotreload::{AtomicTransaction, HotReloadRuntime, StateSnapshot};
use parking_lot::RwLock;
use std::sync::Arc;

extern "C" fn handler_v1() -> i32 {
    1
}

extern "C" fn handler_v2() -> i32 {
    2
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = HotReloadRuntime::new();
    runtime.create_table("handlers");

    let old_ptr = handler_v1 as *const ();
    runtime.get_table("handlers").unwrap().set("handler", old_ptr);
    println!("Registered handler v1 at {:p}", old_ptr);

    let new_ptr = handler_v2 as *const ();
    let replaced = runtime.replace_function("handlers", "handler", new_ptr)?;
    println!("Swapped handler v1 -> v2 (old was {:p})", replaced);

    // Demonstrate transactional rollback of application state that a hot
    // reload might have mutated alongside the function swap.
    let state = Arc::new(RwLock::new(42u32));
    let saved = *state.read();
    let mut tx = AtomicTransaction::new();
    tx.add_snapshot(StateSnapshot::new(state.clone(), saved));

    *state.write() = 100;
    println!("State mutated to {}", *state.read());

    tx.rollback();
    println!("State after rollback: {}", *state.read());
    assert_eq!(*state.read(), saved);

    Ok(())
}
