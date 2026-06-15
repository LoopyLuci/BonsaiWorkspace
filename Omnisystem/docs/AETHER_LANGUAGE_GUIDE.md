# AETHER Language Guide - Distributed Computing

**Build scalable distributed systems with automatic consensus and load balancing**

---

## Table of Contents
1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Nodes and Clusters](#nodes-and-clusters)
4. [Consensus Protocols](#consensus-protocols)
5. [Message Passing](#message-passing)
6. [Data Distribution](#data-distribution)
7. [Load Balancing](#load-balancing)
8. [Fault Tolerance](#fault-tolerance)
9. [Advanced Topics](#advanced-topics)

---

## Introduction

AETHER is a distributed computing language designed for:
- **Consensus**: Automatic Raft, Paxos, or BFT consensus
- **Scalability**: Horizontal scaling with data sharding
- **Reliability**: Automatic failover and recovery
- **Performance**: Low-latency distributed transactions

### Quick Facts
- **Execution**: Distributed JIT with optimization
- **Consensus**: Raft, Paxos, Byzantine Fault Tolerant
- **Replication**: 3-node consensus minimum
- **Sharding**: Hash-based data partitioning
- **Messaging**: Async message-based communication

---

## Getting Started

### Your First Distributed System

```aether
// cluster.ae
use aether::cluster::*
use aether::consensus::*

fun main() {
    // Create cluster
    let mut cluster = Cluster::new()
    
    // Add nodes
    cluster.add_node("node1", "127.0.0.1:5001")
    cluster.add_node("node2", "127.0.0.1:5002")
    cluster.add_node("node3", "127.0.0.1:5003")
    
    // Start Raft consensus
    cluster.start_consensus(ConsensusType::Raft)
    
    println!("Cluster started with 3 nodes")
    
    // Join cluster
    cluster.join()
}
```

### Running the System

```bash
omnisystem run cluster.ae --nodes 3
# Cluster started with 3 nodes
# Node 1 leader: 127.0.0.1:5001
# Node 2 follower: 127.0.0.1:5002
# Node 3 follower: 127.0.0.1:5003
```

---

## Nodes and Clusters

### Node Creation

```aether
use aether::node::*

// Create standalone node
let node = Node::new("node1", "127.0.0.1:5001")

// Configure node
let node = Node::new("node2", "127.0.0.1:5002")
    .with_heartbeat_interval(150)
    .with_election_timeout(300..600)
    .with_max_log_entries(10000)

// Get node ID
let id = node.id()

// Get node state
let state = node.current_state()  // Follower, Candidate, Leader
```

### Cluster Setup

```aether
use aether::cluster::*

// Create cluster
let mut cluster = Cluster::new()
    .with_min_replicas(3)
    .with_heartbeat_timeout(150)

// Add nodes dynamically
cluster.add_node("leader-node", "192.168.1.10:5001")
cluster.add_node("replica-1", "192.168.1.11:5001")
cluster.add_node("replica-2", "192.168.1.12:5001")

// Get cluster info
let leader = cluster.get_leader()
let replicas = cluster.get_replicas()
let member_count = cluster.member_count()
```

### Node Status Monitoring

```aether
// Check node health
if node.is_healthy() {
    println!("Node is healthy")
}

// Get node metrics
let metrics = node.get_metrics()
println!("Uptime: {} seconds", metrics.uptime_seconds)
println!("Messages sent: {}", metrics.messages_sent)
println!("Messages received: {}", metrics.messages_received)

// Watch for state changes
node.on_state_change(|old_state, new_state| {
    println!("State changed: {:?} -> {:?}", old_state, new_state)
})
```

---

## Consensus Protocols

### Raft Consensus

```aether
use aether::consensus::*

// Initialize Raft
let mut node = Node::new("node1", "127.0.0.1:5001")
node.enable_raft()

// For leader: propose new entry
let entry = LogEntry::new(1, "SET key=value")
node.propose_raft_entry(&entry)

// Followers automatically replicate
// Once majority replies, entry is committed

// Check commit index
let commit_index = node.get_commit_index()
println!("Committed entries: {}", commit_index)
```

### Paxos Consensus

```aether
// Initialize Paxos
let mut node = Node::new("node2", "127.0.0.1:5002")
node.enable_paxos()

// Multi-round voting (Prepare -> Promise -> Accept -> Accepted)
let proposal = Proposal::new(1, "transaction data")
let accepted = node.propose_paxos(&proposal)

if accepted {
    println!("Proposal accepted by majority")
}
```

### Byzantine Fault Tolerance

```aether
// Initialize PBFT (Practical Byzantine Fault Tolerant)
let mut cluster = Cluster::new()
    .with_fault_tolerance(FaultTolerance::Byzantine)
    .with_nodes(7)  // Requires 3f+1 nodes (f=2)

// Tolerates up to f=2 Byzantine nodes
cluster.start()

// Client sends request
let request = ClientRequest::new("operation", "data")
let response = cluster.execute(&request)

// Majority (5 of 7) must agree
if response.consensus_reached {
    println!("Transaction committed")
}
```

---

## Message Passing

### Sending Messages

```aether
use aether::messaging::*

// Create message
let msg = Message::new(
    MessageType::Data,
    "target-node",
    "Hello from distributed system"
)

// Send to specific node
node.send_message(&msg)

// Send to all nodes
node.broadcast(&msg)

// Send with timeout
node.send_with_timeout(&msg, Duration::from_secs(5))
```

### Receiving Messages

```aether
// Handle incoming messages
node.on_message(|msg: &Message| {
    match msg.msg_type {
        MessageType::Data => {
            println!("Data: {}", msg.payload)
        },
        MessageType::Request => {
            let response = handle_request(&msg.payload)
            msg.respond_with(&response)
        },
        MessageType::Response => {
            process_response(&msg.payload)
        },
        _ => {}
    }
})
```

### Message Types

```aether
enum MessageType {
    Data,           // Regular data message
    Request,        // RPC request
    Response,       // RPC response
    Heartbeat,      // Leader heartbeat (Raft)
    VoteRequest,    // Vote request (Raft)
    VoteResponse,   // Vote response (Raft)
    AppendEntries,  // Log replication (Raft)
    Prepare,        // Paxos prepare phase
    Promise,        // Paxos promise phase
    Accept,         // Paxos accept phase
    Accepted,       // Paxos accepted phase
}
```

---

## Data Distribution

### Hash-Based Sharding

```aether
use aether::sharding::*

// Create partitioner
let partitioner = Partitioner::new(shard_count: 16)

// Get shard for key
let shard_id = partitioner.get_shard("user:123")  // -> shard 5

// All keys with same hash go to same shard
let shard_id = partitioner.get_shard("user:456")  // -> shard 14

// Consistent hashing for scaling
let partitioner = Partitioner::with_replicas(
    shard_count: 32,
    replica_factor: 3
)

// Key is replicated on 3 shards
let replicas = partitioner.get_replicas("key")
```

### Distributed Data Storage

```aether
use aether::storage::*

// Create distributed store
let store = DistributedStore::new(cluster)
    .with_replication_factor(3)
    .with_consistency_level(ConsistencyLevel::Strong)

// Write with replication
store.put("user:123", "Alice", Durability::Persistent)?

// Read from replica
let value = store.get("user:123")?

// Conditional write
store.compare_and_swap(
    "counter",
    "10",
    "11"
)?

// Batch operations
let batch = WriteBatch::new()
    .put("key1", "value1")
    .put("key2", "value2")
    .delete("key3")

store.write_batch(batch)?
```

### Range Queries

```aether
// Range scan
let results = store.range("user:*")?
for (key, value) in results {
    println!("{} = {}", key, value)
}

// Limited results
let first_10 = store.range_with_limit("item:*", 10)?

// Reverse scan
let reverse = store.range_reverse("log:*")?
```

---

## Load Balancing

### Round-Robin Load Balancing

```aether
use aether::lb::*

// Create load balancer
let mut lb = LoadBalancer::new()
    .with_strategy(LBStrategy::RoundRobin)

// Add backend nodes
lb.add_node("backend1", "192.168.1.10:8080")
lb.add_node("backend2", "192.168.1.11:8080")
lb.add_node("backend3", "192.168.1.12:8080")

// Get next node
let node = lb.next_node()  // Cycles: 1 -> 2 -> 3 -> 1

// Send request
lb.forward_request(request, node)?
```

### Least Connections Load Balancing

```aether
// Create least-connections balancer
let lb = LoadBalancer::new()
    .with_strategy(LBStrategy::LeastConnections)

// Balancer tracks active connections
let node1_conns = lb.get_connections("backend1")  // 5
let node2_conns = lb.get_connections("backend2")  // 2
let node3_conns = lb.get_connections("backend3")  // 8

// Routes to backend2 (fewest connections)
let node = lb.next_node()
```

### Custom Load Balancing

```aether
// Weighted round-robin
let lb = LoadBalancer::new()
    .add_weighted_node("fast-node", 3)    // 3x capacity
    .add_weighted_node("medium-node", 2)  // 2x capacity
    .add_weighted_node("slow-node", 1)    // 1x capacity

// Nodes selected proportionally to weight
let node = lb.next_node()
```

---

## Fault Tolerance

### Automatic Failover

```aether
// Node health check
let health = node.health_check()

if !health.is_healthy() {
    println!("Node unhealthy, initiating failover")
    cluster.trigger_failover(&node.id())
}

// Automatic detection
cluster.on_node_failure(|node_id| {
    println!("Node {} failed", node_id)
    // Automatically re-elects leader if needed
    // Automatically rebalances data
})
```

### Replica Synchronization

```aether
// Check replica lag
let lag = replica.get_replication_lag()
println!("Replica lag: {} ms", lag)

// Catch up slow replica
if lag > Duration::from_secs(5) {
    replica.sync_from_leader()
}

// Monitor replication
cluster.on_replication_complete(|replica_id| {
    println!("Replica {} synchronized", replica_id)
})
```

### Data Recovery

```aether
// Snapshot-based recovery
node.create_snapshot()  // Current state
node.save_snapshot("snapshot.bin")?

// Load from snapshot
let recovered_node = Node::from_snapshot("snapshot.bin")?

// Partial recovery
let missing_entries = node.find_missing_entries()
node.pull_entries_from_leader(&missing_entries)?
```

---

## Advanced Topics

### Distributed Transactions

```aether
use aether::transactions::*

// Two-phase commit
let tx = DistributedTransaction::new()
    .add_operation(Operation::Write("key1", "value1"))
    .add_operation(Operation::Write("key2", "value2"))

// Phase 1: Prepare
if cluster.prepare(&tx)? {
    // Phase 2: Commit
    cluster.commit(&tx)?
} else {
    // Rollback
    cluster.rollback(&tx)?
}
```

### Distributed Locking

```aether
use aether::locking::*

// Acquire distributed lock
let lock = DistributedLock::new("resource-key")
lock.acquire(Duration::from_secs(30))?

// Use locked resource
do_critical_operation()

// Release lock
lock.release()?

// Wait for lock with timeout
if lock.wait_for_timeout(Duration::from_secs(5))? {
    // Lock acquired
}
```

### Service Discovery

```aether
use aether::discovery::*

// Register service
let service = Service::new("api-server")
    .with_address("192.168.1.10:8080")
    .with_tags(["prod", "primary"])
    .with_health_check("http://localhost:8080/health")

cluster.register_service(&service)?

// Discover service
let services = cluster.discover("api-server")?
for service in services {
    println!("{}: {}", service.name, service.address)
}

// Watch for service changes
cluster.on_service_change("database", |service| {
    println!("Service changed: {:?}", service)
})
```

### Distributed Monitoring

```aether
// Collect metrics
let metrics = node.collect_metrics()
println!("CPU: {}%", metrics.cpu_usage)
println!("Memory: {}MB", metrics.memory_usage)
println!("Network: {} Mbps", metrics.network_throughput)

// Cluster-wide metrics
let cluster_metrics = cluster.get_aggregated_metrics()
println!("Total throughput: {} req/sec", cluster_metrics.total_throughput)
println!("P99 latency: {} ms", cluster_metrics.p99_latency)

// Alert on anomalies
cluster.on_anomaly_detected(|metric, value, threshold| {
    println!("Anomaly: {} = {}, threshold = {}", metric, value, threshold)
})
```

---

## Complete Example: Multi-Node Key-Value Store

```aether
use aether::cluster::*
use aether::consensus::*
use aether::sharding::*
use aether::storage::*

fun create_kvstore_cluster(num_nodes: i32) -> Result<Cluster> {
    let mut cluster = Cluster::new()
        .with_min_replicas(3)
        .with_fault_tolerance(FaultTolerance::Raft)
    
    // Create nodes
    for i in 0..num_nodes {
        let port = 5000 + i as u16
        let addr = format!("127.0.0.1:{}", port)
        cluster.add_node(&format!("node{}", i), &addr)
    }
    
    cluster.start()
    Ok(cluster)
}

fun put(cluster: &Cluster, key: string, value: string) -> Result<()> {
    // Replicate to 3 nodes
    let msg = Message::new(
        MessageType::Data,
        "all",
        &format!("PUT {} = {}", key, value)
    )
    
    cluster.broadcast(&msg)
    
    // Wait for consensus
    cluster.wait_for_commit()
    
    Ok(())
}

fun get(cluster: &Cluster, key: string) -> Result<string> {
    // Read from leader
    let leader = cluster.get_leader()
    
    leader.query(&format!("GET {}", key))
}

fun main() -> Result<()> {
    // Create 5-node cluster
    let cluster = create_kvstore_cluster(5)?
    
    // Write operations
    put(&cluster, "user:1", "Alice")?
    put(&cluster, "user:2", "Bob")?
    
    // Read operations
    let name = get(&cluster, "user:1")?
    println!("User 1: {}", name)
    
    // Distributed transactions
    let tx = DistributedTransaction::new()
        .add_operation(Operation::Write("balance:1", "1000"))
        .add_operation(Operation::Write("balance:2", "2000"))
    
    if cluster.prepare(&tx)? {
        cluster.commit(&tx)?
    }
    
    // Monitor cluster
    loop {
        let leader = cluster.get_leader()
        let members = cluster.member_count()
        println!("Leader: {:?}, Members: {}", leader, members)
        
        std::thread::sleep(Duration::from_secs(5))
    }
}
```

---

## Best Practices

✅ **DO**
- Use replication factor ≥ 3
- Monitor replication lag
- Test failure scenarios
- Use consistent hashing
- Implement circuit breakers
- Monitor consensus health
- Set appropriate timeouts
- Log all distributed decisions

❌ **DON'T**
- Use single node for critical data
- Ignore network partitions
- Mix consensus protocols
- Create hot shards
- Disable heartbeats
- Ignore replication lag
- Assume network reliability
- Skip snapshot creation

---

## Performance Tips

1. **Batch operations** to reduce consensus rounds
2. **Pipeline requests** for higher throughput
3. **Use appropriate replication** for consistency needs
4. **Monitor latency** for early warning
5. **Balance shards** to prevent hot spots
6. **Tune timeouts** for your network

---

## Debugging

### Cluster State Inspection

```aether
// Check node state
println!("State: {:?}", node.current_state())
println!("Term: {}", node.current_term())
println!("Voted for: {:?}", node.voted_for())

// Check log
let log = node.get_log()
println!("Log entries: {}", log.len())
println!("Commit index: {}", node.get_commit_index())

// Trace messages
node.enable_message_tracing()
```

---

## See Also
- [API_AETHER.md](API_AETHER.md) - Complete API reference
- [TUTORIAL_DISTRIBUTED.md](TUTORIAL_DISTRIBUTED.md) - Multi-node example
- [AETHER_LANGUAGE_SPECIFICATION.md](AETHER_LANGUAGE_SPECIFICATION.md) - Formal spec

---

**Next**: [TUTORIAL_DISTRIBUTED.md](TUTORIAL_DISTRIBUTED.md) - Build a distributed system
