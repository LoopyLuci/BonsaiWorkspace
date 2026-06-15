# AETHER Runtime API Reference

**Complete API reference for distributed computing and consensus**

---

## Module Overview

The AETHER runtime provides:
- **Nodes & Clusters**: Distributed system orchestration
- **Consensus Protocols**: Raft, Paxos, Byzantine Fault Tolerance
- **Message Passing**: Async request/response patterns
- **Data Sharding**: Consistent hashing and partitioning
- **Load Balancing**: Distribution strategies

---

## Core Types

### Node

**Individual distributed system node**

```rust
pub struct Node {
    id: String,
    address: NetworkAddress,
    state: NodeState,
    current_term: u64,
    voted_for: Option<String>,
}

impl Node {
    pub fn new(id: &str, address: &str) -> Self
    pub fn id(&self) -> &str
    pub fn address(&self) -> &NetworkAddress
    pub fn current_state(&self) -> NodeState
    pub fn current_term(&self) -> u64
    pub fn is_healthy(&self) -> bool
    pub fn get_metrics(&self) -> NodeMetrics
    pub fn enable_raft(&mut self)
    pub fn enable_paxos(&mut self)
    pub fn on_state_change<F>(&self, callback: F) where F: Fn(NodeState, NodeState)
}

pub enum NodeState {
    Follower,
    Candidate,
    Leader,
}

pub struct NodeMetrics {
    pub uptime_seconds: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub last_activity_secs: u64,
}
```

**Example:**
```rust
let node = Node::new("node1", "127.0.0.1:5001")
node.enable_raft()
println!("Node: {}, State: {:?}", node.id(), node.current_state())
```

---

### Cluster

**Multi-node distributed cluster**

```rust
pub struct Cluster {
    nodes: HashMap<String, Node>,
    consensus_type: ConsensusType,
    min_replicas: usize,
}

impl Cluster {
    pub fn new() -> Self
    pub fn with_min_replicas(mut self, count: usize) -> Self
    pub fn with_fault_tolerance(mut self, ft: FaultTolerance) -> Self
    pub fn add_node(&mut self, id: &str, address: &str) -> Result<()>
    pub fn remove_node(&mut self, id: &str) -> Result<()>
    pub fn start_consensus(&mut self, consensus_type: ConsensusType) -> Result<()>
    pub fn start(&mut self) -> Result<()>
    pub fn join(&mut self) -> Result<()>
    pub fn get_leader(&self) -> Option<String>
    pub fn get_replicas(&self) -> Vec<String>
    pub fn member_count(&self) -> usize
    pub fn wait_for_commit(&self) -> Result<()>
    pub fn trigger_failover(&mut self, node_id: &str) -> Result<()>
    pub fn on_node_failure<F>(&mut self, callback: F) where F: Fn(&str)
    pub fn on_replication_complete<F>(&mut self, callback: F) where F: Fn(&str)
}

pub enum ConsensusType {
    Raft,
    Paxos,
    Byzantine,
}

pub enum FaultTolerance {
    Raft,
    Paxos,
    Byzantine,  // Requires 3f+1 nodes
}
```

**Example:**
```rust
let mut cluster = Cluster::new()
    .with_min_replicas(3)

cluster.add_node("node1", "127.0.0.1:5001")?
cluster.add_node("node2", "127.0.0.1:5002")?
cluster.add_node("node3", "127.0.0.1:5003")?

cluster.start_consensus(ConsensusType::Raft)?
cluster.start()?
```

---

### Message

**Distributed system message**

```rust
pub struct Message {
    pub id: String,
    pub from: String,
    pub to: String,
    pub msg_type: MessageType,
    pub payload: Vec<u8>,
    pub timestamp: SystemTime,
}

impl Message {
    pub fn new(msg_type: MessageType, to: &str, payload: &str) -> Self
    pub fn respond_with(&self, response: &[u8]) -> Result<()>
}

pub enum MessageType {
    Data,
    Request,
    Response,
    Heartbeat,
    VoteRequest,
    VoteResponse,
    AppendEntries,
    Prepare,
    Promise,
    Accept,
    Accepted,
}
```

**Example:**
```rust
let msg = Message::new(
    MessageType::Data,
    "target-node",
    "Hello, world!"
)
node.send_message(&msg)?
```

---

### Consensus Implementations

**Raft Consensus**

```rust
pub struct RaftNode {
    log: Vec<LogEntry>,
    commit_index: usize,
    last_applied: usize,
    next_index: HashMap<String, usize>,
    match_index: HashMap<String, usize>,
}

impl RaftNode {
    pub fn new() -> Self
    pub fn append_entry(&mut self, entry: LogEntry) -> Result<()>
    pub fn commit_entries(&mut self) -> Result<()>
    pub fn get_commit_index(&self) -> usize
    pub fn get_log(&self) -> &[LogEntry]
    pub fn propose_raft_entry(&mut self, entry: &LogEntry) -> Result<bool>
}

pub struct LogEntry {
    pub index: u64,
    pub term: u64,
    pub command: String,
}
```

**Paxos Consensus**

```rust
pub struct PaxosNode {
    proposal_number: u64,
    accepted_proposal: Option<Proposal>,
    accepted_value: Option<Vec<u8>>,
}

impl PaxosNode {
    pub fn new() -> Self
    pub fn propose_paxos(&mut self, proposal: &Proposal) -> Result<bool>
    pub fn prepare_phase(&mut self, proposal_num: u64) -> Result<Promise>
    pub fn accept_phase(&mut self, proposal: &Proposal) -> Result<bool>
}

pub struct Proposal {
    pub number: u64,
    pub value: Vec<u8>,
}
```

---

### Distributed Storage

**Replicated key-value store**

```rust
pub struct DistributedStore {
    cluster: Arc<Cluster>,
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    replication_factor: usize,
    consistency_level: ConsistencyLevel,
}

impl DistributedStore {
    pub fn new(cluster: Arc<Cluster>) -> Self
    pub fn with_replication_factor(mut self, factor: usize) -> Self
    pub fn with_consistency_level(mut self, level: ConsistencyLevel) -> Self
    pub fn put(&self, key: &str, value: &str, durability: Durability) -> Result<()>
    pub fn get(&self, key: &str) -> Result<String>
    pub fn delete(&self, key: &str) -> Result<()>
    pub fn compare_and_swap(&self, key: &str, old: &str, new: &str) -> Result<bool>
    pub fn write_batch(&self, batch: WriteBatch) -> Result<()>
    pub fn range(&self, pattern: &str) -> Result<Vec<(String, String)>>
    pub fn range_with_limit(&self, pattern: &str, limit: usize) -> Result<Vec<(String, String)>>
    pub fn range_reverse(&self, pattern: &str) -> Result<Vec<(String, String)>>
}

pub enum ConsistencyLevel {
    Strong,           // All replicas consistent
    Eventual,         // Replicas converge over time
    ReadYourWrites,   // Client sees own writes
}

pub enum Durability {
    Persistent,  // Committed to disk
    Memory,      // In-memory only
}

pub struct WriteBatch {
    operations: Vec<BatchOp>,
}

impl WriteBatch {
    pub fn new() -> Self
    pub fn put(mut self, key: &str, value: &str) -> Self
    pub fn delete(mut self, key: &str) -> Self
}
```

**Example:**
```rust
let store = DistributedStore::new(Arc::new(cluster))
    .with_replication_factor(3)
    .with_consistency_level(ConsistencyLevel::Strong)

store.put("user:1", "Alice", Durability::Persistent)?
let value = store.get("user:1")?
```

---

### Sharding & Partitioning

**Distribute data across nodes**

```rust
pub struct Partitioner {
    shard_count: usize,
    replicas: usize,
    ring: ConsistentHashRing,
}

impl Partitioner {
    pub fn new(shard_count: usize) -> Self
    pub fn with_replicas(shard_count: usize, replica_factor: usize) -> Self
    pub fn get_shard(&self, key: &str) -> u32
    pub fn get_replicas(&self, key: &str) -> Vec<u32>
    pub fn add_node(&mut self, node_id: &str) -> Result<()>
    pub fn remove_node(&mut self, node_id: &str) -> Result<()>
}
```

**Example:**
```rust
let partitioner = Partitioner::new(16)
let shard = partitioner.get_shard("user:123")
let replicas = partitioner.get_replicas("user:123")
```

---

### Load Balancing

**Distribute traffic across nodes**

```rust
pub struct LoadBalancer {
    nodes: Vec<BackendNode>,
    strategy: LBStrategy,
    current_index: usize,
}

impl LoadBalancer {
    pub fn new() -> Self
    pub fn with_strategy(mut self, strategy: LBStrategy) -> Self
    pub fn add_node(&mut self, id: &str, address: &str) -> Result<()>
    pub fn add_weighted_node(&mut self, id: &str, weight: u32) -> Result<()>
    pub fn remove_node(&mut self, id: &str) -> Result<()>
    pub fn next_node(&mut self) -> Option<&BackendNode>
    pub fn forward_request(&self, req: &Request, node: &BackendNode) -> Result<Response>
    pub fn get_connections(&self, node_id: &str) -> usize
}

pub enum LBStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRoundRobin,
    IpHash,
    Random,
}

pub struct BackendNode {
    id: String,
    address: String,
    weight: u32,
    active_connections: AtomicU32,
}
```

**Example:**
```rust
let mut lb = LoadBalancer::new()
    .with_strategy(LBStrategy::RoundRobin)

lb.add_node("backend1", "192.168.1.10:8080")?
lb.add_node("backend2", "192.168.1.11:8080")?

let node = lb.next_node()?
```

---

### Distributed Transactions

**Multi-phase commits**

```rust
pub struct DistributedTransaction {
    id: String,
    operations: Vec<Operation>,
    state: TransactionState,
}

impl DistributedTransaction {
    pub fn new() -> Self
    pub fn add_operation(mut self, op: Operation) -> Self
    pub fn id(&self) -> &str
    pub fn state(&self) -> TransactionState
}

pub enum Operation {
    Read(String),
    Write(String, String),
    Delete(String),
}

pub enum TransactionState {
    Pending,
    Preparing,
    Prepared,
    Committing,
    Committed,
    RolledBack,
}
```

**Example:**
```rust
let tx = DistributedTransaction::new()
    .add_operation(Operation::Write("key1", "value1"))
    .add_operation(Operation::Write("key2", "value2"))

if cluster.prepare(&tx)? {
    cluster.commit(&tx)?
} else {
    cluster.rollback(&tx)?
}
```

---

### Distributed Locking

**Coordinate access to shared resources**

```rust
pub struct DistributedLock {
    key: String,
    owner: String,
    expires_at: SystemTime,
}

impl DistributedLock {
    pub fn new(key: &str) -> Self
    pub fn acquire(&self, timeout: Duration) -> Result<()>
    pub fn release(&self) -> Result<()>
    pub fn is_held(&self) -> bool
    pub fn wait_for_timeout(&self, timeout: Duration) -> Result<bool>
}
```

**Example:**
```rust
let lock = DistributedLock::new("resource")
lock.acquire(Duration::from_secs(30))?
do_critical_work()
lock.release()?
```

---

### Service Discovery

**Locate and monitor services**

```rust
pub struct ServiceRegistry {
    services: HashMap<String, Vec<Service>>,
}

impl ServiceRegistry {
    pub fn register(&mut self, service: Service) -> Result<()>
    pub fn deregister(&mut self, service_id: &str) -> Result<()>
    pub fn discover(&self, service_name: &str) -> Result<Vec<Service>>
    pub fn on_service_change<F>(&mut self, service_name: &str, callback: F)
        where F: Fn(&Service)
}

pub struct Service {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
    pub tags: Vec<String>,
    pub health_check: Option<HealthCheck>,
    pub metadata: HashMap<String, String>,
}

impl Service {
    pub fn new(name: &str) -> Self
    pub fn with_address(mut self, address: &str) -> Self
    pub fn with_tags(mut self, tags: Vec<&str>) -> Self
    pub fn with_health_check(mut self, check: HealthCheck) -> Self
}
```

---

## Error Types

### DistributedError

**Consensus and distribution errors**

```rust
pub enum DistributedError {
    QuorumNotReached,
    ConsensusTimeout,
    NetworkPartitioned,
    ReplicationFailed,
    DataInconsistent,
    NodeDown(String),
    ShardNotFound,
    TransactionAborted,
    LockTimeout,
}
```

---

## Usage Patterns

### Cluster Setup

```rust
let mut cluster = Cluster::new()
    .with_min_replicas(3)

for i in 0..5 {
    let port = 5000 + i as u16
    cluster.add_node(
        &format!("node{}", i),
        &format!("127.0.0.1:{}", port)
    )?
}

cluster.start_consensus(ConsensusType::Raft)?
cluster.start()?
```

### Distributed Storage

```rust
let store = DistributedStore::new(Arc::new(cluster))
    .with_replication_factor(3)

// Write with strong consistency
store.put("config", "value", Durability::Persistent)?

// Batch operations
let batch = WriteBatch::new()
    .put("key1", "val1")
    .put("key2", "val2")

store.write_batch(batch)?
```

---

## Examples

### Multi-Node Key-Value Store

```rust
let mut cluster = Cluster::new()
for i in 0..3 {
    cluster.add_node(&format!("node{}", i), &format!("127.0.0.1:{}", 5000+i))?
}
cluster.start()?

let store = DistributedStore::new(Arc::new(cluster))?
store.put("users:1", "Alice", Durability::Persistent)?
let value = store.get("users:1")?
```

---

## Testing

### Cluster Tests

```rust
#[test]
fn test_cluster_formation() {
    let mut c = Cluster::new()
    c.add_node("n1", "127.0.0.1:5001").unwrap()
    c.add_node("n2", "127.0.0.1:5002").unwrap()
    assert_eq!(c.member_count(), 2)
}

#[test]
fn test_leader_election() {
    let mut c = Cluster::new()
    c.start_consensus(ConsensusType::Raft).unwrap()
    assert!(c.get_leader().is_some())
}
```

---

## Performance Notes

- Consensus requires **majority quorum** (n/2 + 1)
- Byzantine FT requires **3f+1 nodes** (f = faulty)
- Use **batching** to improve throughput
- Monitor **replication lag** for performance
- Adjust **timeout parameters** for network conditions

---

## See Also
- [AETHER_LANGUAGE_GUIDE.md](AETHER_LANGUAGE_GUIDE.md) - Language tutorial
- [TUTORIAL_DISTRIBUTED.md](TUTORIAL_DISTRIBUTED.md) - Distributed example
- [AETHER_LANGUAGE_SPECIFICATION.md](AETHER_LANGUAGE_SPECIFICATION.md) - Formal spec

---

**Last Updated**: 2026-06-15
