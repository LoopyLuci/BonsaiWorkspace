# AETHER Language Specification - Complete Reference

**Formal specification for AETHER distributed systems language**

---

## Language Overview

**AETHER** is a statically-typed distributed systems language with:
- Multi-node coordination primitives
- Built-in consensus algorithms (Raft, Paxos, PBFT)
- Fault tolerance and automatic failover
- Distributed transactions with ACID guarantees
- Type-safe RPC and message passing
- Zero-downtime updates
- Byzantine fault detection

---

## Core Concepts

### Nodes & Clusters

```aether
// Define a node in the cluster
node Server {
    id: string,
    address: string,
    port: u16,
}

// Define cluster
cluster MyCluster {
    nodes: Vec<Server>,
    min_quorum: usize,
    heartbeat_interval: Duration,
}

fun create_cluster() -> MyCluster {
    MyCluster {
        nodes: vec![
            Server { id: "node1", address: "192.168.1.1", port: 5000 },
            Server { id: "node2", address: "192.168.1.2", port: 5000 },
            Server { id: "node3", address: "192.168.1.3", port: 5000 },
        ],
        min_quorum: 2,
        heartbeat_interval: Duration::from_millis(100),
    }
}
```

### Consensus Algorithms

```aether
// Raft consensus
type RaftConsensus {
    term: u64,
    voted_for: Option<string>,
    log: Vec<LogEntry>,
    commit_index: usize,
    last_applied: usize,
}

// Paxos consensus
type PaxosConsensus {
    proposer_id: string,
    proposal_number: u64,
    accepted_proposals: Map<u64, Value>,
    accepted_proposal_number: u64,
}

// Byzantine Fault Tolerant (PBFT)
type PBFTConsensus {
    view_number: u64,
    sequence_number: u64,
    pre_prepare_messages: Vec<PrePrepare>,
    prepare_messages: Vec<Prepare>,
    commit_messages: Vec<Commit>,
}
```

---

## Distributed State

### State Management

```aether
// Replicated state machine
type StateMachine {
    state: Map<string, Value>,
    version: u64,
    last_applied_index: usize,
}

impl StateMachine {
    fun apply(mut self, command: Command) -> Result<Value> {
        match command {
            Command::Set(key, value) => {
                self.state.insert(key, value.clone())
                self.version += 1
                Ok(value)
            },
            Command::Get(key) => {
                Ok(self.state.get(&key).unwrap_or(Value::Null))
            },
            Command::Delete(key) => {
                self.state.remove(&key)
                self.version += 1
                Ok(Value::Null)
            },
        }
    }
}

// Versioned snapshots
type Snapshot {
    term: u64,
    index: usize,
    state: StateMachine,
    timestamp: u64,
}

fun create_snapshot(state: &StateMachine) -> Snapshot {
    Snapshot {
        term: state.version,
        index: 0,
        state: state.clone(),
        timestamp: system_time(),
    }
}
```

### Replication

```aether
// Log entry replication
type LogEntry {
    index: usize,
    term: u64,
    command: Command,
    applied: bool,
}

// Replicate entry across cluster
fun replicate_entry(entry: LogEntry, cluster: &Cluster) -> Result<()> {
    let replications = cluster.nodes.len()
    let mut acks = 0
    
    for node in &cluster.nodes {
        if send_append_entries(node, &entry)? {
            acks += 1
        }
    }
    
    if acks >= cluster.min_quorum {
        Ok(())
    } else {
        Err("Failed to replicate to quorum")
    }
}

// Handle replication failures with retry
fun replicate_with_retry(
    entry: LogEntry,
    cluster: &Cluster,
    max_retries: u32
) -> Result<()> {
    for attempt in 0..max_retries {
        match replicate_entry(entry.clone(), cluster) {
            Ok(()) => return Ok(()),
            Err(_) => {
                sleep(Duration::from_millis(100 * 2.pow(attempt)))
                continue
            }
        }
    }
    Err("Replication failed after max retries")
}
```

---

## RPC & Messaging

### Type-Safe RPC

```aether
// Define RPC service
service KVStore {
    rpc get(key: string) -> (Value);
    rpc set(key: string, value: Value) -> ();
    rpc delete(key: string) -> ();
    rpc scan(prefix: string) -> (Vec<(string, Value)>);
}

// Implement service
impl KVStore for Server {
    async fun get(self, key: string) -> Result<Value> {
        Ok(self.state.get(&key).unwrap_or(Value::Null))
    }
    
    async fun set(mut self, key: string, value: Value) -> Result<()> {
        self.replicate(Command::Set(key, value)).await
    }
    
    async fun delete(mut self, key: string) -> Result<()> {
        self.replicate(Command::Delete(key)).await
    }
    
    async fun scan(self, prefix: string) -> Result<Vec<(string, Value)>> {
        let results = self.state.iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .collect()
        Ok(results)
    }
}

// RPC client
let client = RpcClient::connect("192.168.1.1:5000").await?
let value = client.get("key1").await?
client.set("key2", Value::String("hello")).await?
```

### Message Passing

```aether
// Define message types
enum Message {
    AppendEntries {
        term: u64,
        leader_id: string,
        prev_log_index: usize,
        prev_log_term: u64,
        entries: Vec<LogEntry>,
        leader_commit: usize,
    },
    RequestVote {
        term: u64,
        candidate_id: string,
        last_log_index: usize,
        last_log_term: u64,
    },
    ClientRequest {
        command: Command,
        client_id: string,
        sequence_number: u64,
    },
}

// Message handler
fun handle_message(msg: Message, node: &mut Node) -> Result<Message> {
    match msg {
        Message::AppendEntries { term, leader_id, entries, .. } => {
            if term < node.current_term {
                return Ok(Message::AppendEntriesResponse { success: false })
            }
            
            node.current_term = term
            node.voted_for = None
            
            for entry in entries {
                node.log.push(entry)
            }
            
            Ok(Message::AppendEntriesResponse { success: true })
        },
        Message::RequestVote { term, candidate_id, .. } => {
            if term < node.current_term {
                return Ok(Message::RequestVoteResponse { vote_granted: false })
            }
            
            if node.voted_for.is_none() || node.voted_for == Some(candidate_id.clone()) {
                node.voted_for = Some(candidate_id)
                Ok(Message::RequestVoteResponse { vote_granted: true })
            } else {
                Ok(Message::RequestVoteResponse { vote_granted: false })
            }
        },
        _ => Ok(Message::Error("Unknown message")),
    }
}
```

---

## Transactions

### Distributed Transactions

```aether
// Transaction definition
type Transaction {
    id: string,
    operations: Vec<Operation>,
    state: TransactionState,
    timestamp: u64,
}

enum TransactionState {
    Pending,
    Committed,
    Aborted,
    RolledBack,
}

enum Operation {
    Read(string),
    Write(string, Value),
    Delete(string),
}

// Two-Phase Commit
fun two_phase_commit(
    tx: &mut Transaction,
    coordinator: &Node,
    participants: &[Node]
) -> Result<()> {
    // Phase 1: Prepare
    let mut votes = vec![]
    for participant in participants {
        let vote = participant.prepare(tx).await?
        votes.push(vote)
    }
    
    let can_commit = votes.iter().all(|v| *v == Vote::Yes)
    
    // Phase 2: Commit/Abort
    if can_commit {
        tx.state = TransactionState::Committed
        for participant in participants {
            participant.commit(tx).await?
        }
        Ok(())
    } else {
        tx.state = TransactionState::Aborted
        for participant in participants {
            participant.abort(tx).await?
        }
        Err("Transaction aborted")
    }
}

// MVCC for concurrent transactions
type MVCCVersion {
    value: Value,
    version: u64,
    writer_id: string,
}

type MVCCData {
    key: string,
    versions: Vec<MVCCVersion>,
}

fun read_version(data: &MVCCData, version: u64) -> Option<Value> {
    data.versions
        .iter()
        .filter(|v| v.version <= version)
        .max_by_key(|v| v.version)
        .map(|v| v.value.clone())
}
```

---

## Fault Tolerance

### Failure Detection

```aether
// Heartbeat monitoring
fun monitor_node_health(node: &Node, timeout: Duration) -> NodeHealth {
    match node.last_heartbeat.elapsed() {
        elapsed if elapsed < timeout => NodeHealth::Healthy,
        elapsed if elapsed < timeout * 2 => NodeHealth::Slow,
        _ => NodeHealth::Dead,
    }
}

// Health check
fun health_check(cluster: &Cluster) -> Map<string, NodeHealth> {
    let mut health = Map::new()
    let timeout = Duration::from_secs(1)
    
    for node in &cluster.nodes {
        let status = monitor_node_health(node, timeout)
        health.insert(node.id.clone(), status)
    }
    
    health
}

// Automatic failover
async fun automatic_failover(
    cluster: &mut Cluster,
    failed_node: &Node
) -> Result<()> {
    // Remove failed node
    cluster.nodes.retain(|n| n.id != failed_node.id)
    
    if cluster.nodes.len() < cluster.min_quorum {
        return Err("Cluster size below quorum")
    }
    
    // Elect new leader if needed
    if failed_node.id == cluster.leader_id {
        let new_leader = cluster.elect_leader().await?
        cluster.leader_id = new_leader
    }
    
    // Re-balance data
    cluster.rebalance().await?
    
    Ok(())
}
```

### Byzantine Fault Tolerance

```aether
// PBFT with Byzantine tolerance
type ByzantineNode {
    id: string,
    view_number: u64,
    is_faulty: bool,
}

fun is_byzantine_safe(votes: &[(string, bool)], f: usize) -> bool {
    let true_count = votes.iter().filter(|(_, v)| *v).count()
    true_count > 2 * f  // Requires 2f+1 agreement out of 3f+1 nodes
}

// Byzantine agreement protocol
async fun byzantine_agreement(
    proposal: Value,
    nodes: &[ByzantineNode],
    rounds: u32
) -> Result<Value> {
    let f = nodes.len() / 3  // Maximum Byzantine nodes
    
    for round in 0..rounds {
        let mut votes = vec![]
        
        for node in nodes {
            let vote = if node.is_faulty {
                random_value()
            } else {
                proposal.clone()
            }
            votes.push((node.id.clone(), vote == proposal))
        }
        
        if is_byzantine_safe(&votes, f) {
            return Ok(proposal)
        }
    }
    
    Err("Byzantine agreement failed")
}
```

---

## Sharding & Partitioning

### Data Sharding

```aether
// Consistent hashing for sharding
fun hash_key(key: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher
    let mut hasher = DefaultHasher::new()
    key.hash(&mut hasher)
    hasher.finish()
}

type ShardKey {
    key: string,
    shard_id: usize,
}

fun determine_shard(key: &str, num_shards: usize) -> usize {
    (hash_key(key) as usize) % num_shards
}

// Shard rebalancing
async fun rebalance_shards(
    cluster: &mut Cluster,
    num_shards: usize
) -> Result<()> {
    for shard_id in 0..num_shards {
        let target_nodes = cluster.select_shard_replicas(shard_id, count: 3)?
        
        // Migrate shard data
        let data = cluster.get_shard_data(shard_id).await?
        
        for node in target_nodes {
            node.store_shard(shard_id, &data).await?
        }
    }
    
    Ok(())
}
```

### Range Partitioning

```aether
// Range-based partitioning
type PartitionKey {
    start: string,
    end: string,
    partition_id: usize,
}

fun find_partition(key: &str, partitions: &[PartitionKey]) -> Option<usize> {
    partitions.iter()
        .find(|p| key >= &p.start && key < &p.end)
        .map(|p| p.partition_id)
}

// Dynamic partition splitting
async fun split_partition(
    partition: &Partition,
    split_point: &str,
    cluster: &Cluster
) -> Result<(Partition, Partition)> {
    let data = partition.get_data().await?
    
    let left_data: Vec<_> = data.iter()
        .filter(|(k, _)| k < split_point)
        .collect()
    
    let right_data: Vec<_> = data.iter()
        .filter(|(k, _)| k >= split_point)
        .collect()
    
    let left_partition = Partition::new(partition.start.clone(), split_point.to_string())
    let right_partition = Partition::new(split_point.to_string(), partition.end.clone())
    
    Ok((left_partition, right_partition))
}
```

---

## Service Discovery

### Service Registry

```aether
// Service registry
type ServiceRegistry {
    services: Map<string, Vec<ServiceEndpoint>>,
    ttl: Duration,
}

type ServiceEndpoint {
    host: string,
    port: u16,
    health: ServiceHealth,
    metadata: Map<string, string>,
}

enum ServiceHealth {
    Healthy,
    Unhealthy,
    Unknown,
}

impl ServiceRegistry {
    async fun register_service(
        mut self,
        service_name: string,
        endpoint: ServiceEndpoint
    ) -> Result<()> {
        self.services
            .entry(service_name)
            .or_insert_with(Vec::new)
            .push(endpoint)
        Ok(())
    }
    
    async fun discover_service(
        self,
        service_name: string
    ) -> Result<Vec<ServiceEndpoint>> {
        Ok(self.services
            .get(&service_name)
            .cloned()
            .unwrap_or_default())
    }
    
    async fun deregister_service(
        mut self,
        service_name: string,
        host: string,
        port: u16
    ) -> Result<()> {
        if let Some(endpoints) = self.services.get_mut(&service_name) {
            endpoints.retain(|e| !(e.host == host && e.port == port))
        }
        Ok(())
    }
}

// Health checking and deregistration
async fun health_check_loop(
    mut registry: ServiceRegistry,
    interval: Duration
) {
    loop {
        sleep(interval).await
        
        for (service_name, endpoints) in registry.services.iter_mut() {
            for endpoint in endpoints.iter_mut() {
                match check_health(&endpoint).await {
                    Ok(true) => endpoint.health = ServiceHealth::Healthy,
                    Ok(false) => endpoint.health = ServiceHealth::Unhealthy,
                    Err(_) => endpoint.health = ServiceHealth::Unknown,
                }
            }
            
            endpoints.retain(|e| e.health != ServiceHealth::Unhealthy)
        }
    }
}
```

---

## Load Balancing

### Balancing Strategies

```aether
enum LoadBalanceStrategy {
    RoundRobin,
    LeastConnections,
    LeastLoad,
    Random,
    Consistent,
}

// Load balancer
type LoadBalancer {
    strategy: LoadBalanceStrategy,
    nodes: Vec<Node>,
    current_index: usize,
}

impl LoadBalancer {
    fun select_node(mut self) -> Result<Node> {
        match self.strategy {
            LoadBalanceStrategy::RoundRobin => {
                let node = self.nodes[self.current_index].clone()
                self.current_index = (self.current_index + 1) % self.nodes.len()
                Ok(node)
            },
            LoadBalanceStrategy::LeastConnections => {
                self.nodes.iter()
                    .min_by_key(|n| n.active_connections)
                    .cloned()
                    .ok_or("No available nodes")
            },
            LoadBalanceStrategy::LeastLoad => {
                self.nodes.iter()
                    .min_by_key(|n| n.cpu_usage + n.memory_usage)
                    .cloned()
                    .ok_or("No available nodes")
            },
            _ => Ok(self.nodes[random() % self.nodes.len()].clone()),
        }
    }
}
```

---

## Next Steps

- [AXIOM_LANGUAGE_SPECIFICATION.md](AXIOM_LANGUAGE_SPECIFICATION.md)
- [PERFORMANCE_BENCHMARKS.md](PERFORMANCE_BENCHMARKS.md)
- [SECURITY_MODEL.md](SECURITY_MODEL.md)

---

**AETHER Specification** - Complete distributed systems language reference!
