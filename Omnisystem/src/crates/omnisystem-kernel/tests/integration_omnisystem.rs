//! Integration tests exercising the kernel modules together.
//!
//! The archived version of this file only printed marketing checkmarks
//! and asserted hardcoded booleans/counts unrelated to the crate's
//! actual API (e.g. "Phase 2: Polyglot bindings (5 languages)" with no
//! polyglot code anywhere in this crate) - replaced with tests that
//! actually drive the real process/memory/scheduling/IPC/capability
//! APIs together.

use omnisystem_kernel::*;

#[test]
fn test_process_memory_and_scheduling_together() {
    let pm = ProcessManager::new();
    let mm = MemoryManager::new().unwrap();
    let scheduler = Scheduler::new();

    let process = pm.create_process(None).unwrap();
    let thread = pm.create_thread(process.id).unwrap();

    // Allocate physical pages for the new process.
    let pages = mm.allocate_pages(4).unwrap();
    assert_eq!(pages.len(), 4);
    assert_eq!(mm.get_stats().allocated_frames, 4);

    // Schedule the thread and pull it back off the ready queue.
    scheduler.add_thread(thread.clone()).unwrap();
    let next = scheduler.schedule_next().unwrap();
    assert_eq!(next.id, thread.id);
    assert_eq!(scheduler.get_current_thread().unwrap().id, thread.id);
}

#[test]
fn test_process_hierarchy_and_termination() {
    let pm = ProcessManager::new();

    let parent = pm.create_process(None).unwrap();
    let child = pm.create_process(Some(parent.id)).unwrap();
    assert_eq!(child.parent_pid, Some(parent.id));
    assert_eq!(pm.process_count(), 2);

    pm.terminate_process(child.id).unwrap();
    assert_eq!(pm.process_count(), 1);
    assert!(pm.get_process(child.id).is_none());
}

#[test]
fn test_ipc_channel_between_two_processes() {
    let pm = ProcessManager::new();
    let ipc = IPCManager::new();

    let sender = pm.create_process(None).unwrap();
    let receiver = pm.create_process(None).unwrap();

    let channel = ipc.create_channel(sender.id, receiver.id).unwrap();
    ipc.send_message(channel.id, sender.id, b"hello kernel".to_vec())
        .unwrap();

    let received = ipc.recv_message(channel.id).unwrap().unwrap();
    assert_eq!(received.data, b"hello kernel");
    assert_eq!(received.sender_pid, sender.id);
    assert_eq!(received.receiver_pid, receiver.id);
}

#[test]
fn test_capability_gated_process_access() {
    let pm = ProcessManager::new();
    let capabilities = CapabilityManager::new();

    let process = pm.create_process(None).unwrap();
    capabilities.create_process_capabilities(process.id).unwrap();

    // No capability granted yet: check should report false.
    assert!(!capabilities
        .check_capability(process.id, Capability::MemoryWrite)
        .unwrap());

    capabilities
        .grant_capability(process.id, Capability::MemoryWrite)
        .unwrap();
    assert!(capabilities
        .check_capability(process.id, Capability::MemoryWrite)
        .unwrap());
}

#[test]
fn test_memory_exhaustion_is_reported_as_error() {
    let mm = MemoryManager::new().unwrap();
    let stats = mm.get_stats();

    // Request far more pages than physically available.
    let result = mm.allocate_pages(stats.total_frames as usize + 1);
    assert!(result.is_err());
}

#[test]
fn test_semaphore_coordinates_real_shared_state() {
    use std::sync::Arc;
    use std::thread;

    let sem = Arc::new(Semaphore::new(1));
    let counter = Arc::new(parking_lot::Mutex::new(0));

    let mut handles = vec![];
    for _ in 0..4 {
        let sem = Arc::clone(&sem);
        let counter = Arc::clone(&counter);
        handles.push(thread::spawn(move || {
            sem.wait();
            *counter.lock() += 1;
            sem.signal();
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    assert_eq!(*counter.lock(), 4);
    assert_eq!(sem.value(), 1);
}
