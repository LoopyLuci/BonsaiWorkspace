//! titan-core CLI: exercises the persistent collections, concurrency
//! primitives, and dynamic-scope Var against the real implementations.

use titan_core::{Agent, Atom, PersistentHashMap, PersistentVector, Ref, Var};

fn main() {
    // Persistent vector: pushing never mutates the original.
    let v1 = PersistentVector::<i32>::new().push(1).push(2).push(3);
    let v2 = v1.push(4);
    println!("v1 = {:?} (len {})", v1.iter().collect::<Vec<_>>(), v1.len());
    println!("v2 = {:?} (len {}), v1 unchanged: {}", v2.iter().collect::<Vec<_>>(), v2.len(), v1.len() == 3);

    // Persistent hashmap: same immutability guarantee.
    let m1 = PersistentHashMap::<&str, i32>::new().insert("a", 1).insert("b", 2);
    let m2 = m1.insert("c", 3);
    println!("m1 has {} entries, m2 has {} entries", m1.len(), m2.len());

    // Atom: atomic in-place swap.
    let counter = Atom::new(0);
    for _ in 0..5 {
        counter.swap(|x| x + 1);
    }
    println!("atom counter = {}", counter.deref());

    // Ref: alter, shared across clones.
    let balance = Ref::new(100);
    let balance_clone = balance.clone();
    balance.alter(|x| x - 30);
    println!("ref balance = {} (seen via clone: {})", balance.deref(), balance_clone.deref());

    // Agent: send an update.
    let agent = Agent::new(vec![1, 2, 3]);
    agent.send(|mut v| {
        v.push(4);
        v
    });
    println!("agent state = {:?}", agent.deref());

    // Var: dynamic scoping restores the old value after the binding ends.
    let log_level = Var::new("info".to_string());
    println!("log_level before = {}", log_level.deref());
    let result = log_level.bind("debug".to_string(), || {
        format!("inside binding: log_level = {}", log_level.deref())
    });
    println!("{result}");
    println!("log_level after = {}", log_level.deref());
}
