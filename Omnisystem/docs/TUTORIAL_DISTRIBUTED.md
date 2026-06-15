# Tutorial: Build a Distributed System with AETHER

**Complete walkthrough building a fault-tolerant distributed database cluster**

---

## Overview

We'll build a distributed system that:
- Creates a 5-node cluster
- Implements Raft consensus
- Stores data with replication
- Handles node failures automatically
- Provides consistent reads/writes
- Monitors cluster health

**Time**: 60-90 minutes  
**Prerequisites**: AETHER Language Guide, API_AETHER.md  
**Difficulty**: Advanced

---

## Step 1: Project Setup

### Create project structure

```bash
mkdir omnisystem-distributed-db
cd omnisystem-distributed-db
touch main.ae
mkdir logs
```

### Create main.ae

```aether
// main.ae - Distributed Database with AETHER

use aether::cluster::*
use aether::consensus::*
use aether::storage::*
use aether::discovery::*

fun main() -> Result<(), String> {
    println!("Distributed Database with AETHER")
    
    // Create cluster
    let mut cluster = create_cluster()?
    
    // Start nodes
    cluster.start()?
    
    // Run server loop
    run_server(&cluster)?
    
    Ok(())
}

fun create_cluster() -> Result<Cluster, String> {
    let mut cluster = Cluster::new()
        .with_min_replicas(3)
        .with_fault_tolerance(FaultTolerance::Raft)
    
    // Add nodes
    for i in 0..5 {
        let port = 5000 + i as u16
        cluster.add_node(
            &format!("node{}", i),
            &format!("127.0.0.1:{}", port)
        )?
    }
    
    println!("Cluster created with 5 nodes")
    Ok(cluster)
}
```

---

## Step 2: Initialize Cluster

### Start Raft consensus

```aether
fun initialize_raft(cluster: &mut Cluster) -> Result<(), String> {
    // Start Raft consensus
    cluster.start_consensus(ConsensusType::Raft)?
    
    // Wait for leader election
    println!("Waiting for leader election...")
    std::thread::sleep(Duration::from_secs(2))
    
    match cluster.get_leader() {
        Some(leader) => println!("Leader elected: {}", leader),
        None => return Err("No leader elected".to_string()),
    }
    
    Ok(())
}
```

### Monitor cluster health

```aether
fun monitor_cluster(cluster: &Cluster) -> Result<(), String> {
    println!("\n=== Cluster Status ===")
    
    let leader = cluster.get_leader()
    println!("Leader: {:?}", leader)
    
    let members = cluster.get_replicas()
    println!("Members: {}", members.len())
    
    for member in members {
        println!("  - {}", member)
    }
    
    Ok(())
}
```

---

## Step 3: Distributed Storage

### Implement key-value store

```aether
fun create_store(cluster: Arc<Cluster>) -> DistributedStore {
    DistributedStore::new(cluster)
        .with_replication_factor(3)
        .with_consistency_level(ConsistencyLevel::Strong)
}
```

### Write operations

```aether
fun write_data(
    store: &DistributedStore,
    key: &str,
    value: &str
) -> Result<(), String> {
    store.put(key, value, Durability::Persistent)
        .map_err(|e| format!("Write failed: {:?}", e))
}

fun batch_write(
    store: &DistributedStore,
    operations: Vec<(&str, &str)>
) -> Result<(), String> {
    let batch = operations.iter().fold(
        WriteBatch::new(),
        |b, (k, v)| b.put(k, v)
    )
    
    store.write_batch(batch)
        .map_err(|e| format!("Batch write failed: {:?}", e))
}
```

### Read operations

```aether
fun read_data(
    store: &DistributedStore,
    key: &str
) -> Result<String, String> {
    store.get(key)
        .map_err(|e| format!("Read failed: {:?}", e))
}

fun range_scan(
    store: &DistributedStore,
    pattern: &str
) -> Result<Vec<(String, String)>, String> {
    store.range(pattern)
        .map_err(|e| format!("Range scan failed: {:?}", e))
}
```

---

## Step 4: Fault Tolerance

### Handle node failures

```aether
fun setup_failure_handling(cluster: &mut Cluster) {
    cluster.on_node_failure(|node_id| {
        println!("⚠️ Node {} failed", node_id)
        println!("Triggering automatic failover...")
    })
}

fun trigger_failover(
    cluster: &mut Cluster,
    failed_node: &str
) -> Result<(), String> {
    println!("Initiating failover for {}", failed_node)
    
    cluster.trigger_failover(failed_node)?
    
    // Wait for new leader
    std::thread::sleep(Duration::from_secs(1))
    
    match cluster.get_leader() {
        Some(new_leader) => {
            println!("New leader: {}", new_leader)
            Ok(())
        },
        None => Err("Failover failed".to_string()),
    }
}
```

### Replica synchronization

```aether
fun sync_replicas(cluster: &Cluster) -> Result<(), String> {
    println!("Synchronizing replicas...")
    
    // Check replication lag
    for replica_id in cluster.get_replicas() {
        // Monitor lag in real application
    }
    
    Ok(())
}
```

---

## Step 5: Data Sharding

### Hash-based partitioning

```aether
fun create_partitioner() -> Partitioner {
    Partitioner::new(16)  // 16 shards
        .with_replicas(3)  // 3 replicas per shard
}

fun get_shard(partitioner: &Partitioner, key: &str) -> u32 {
    partitioner.get_shard(key)
}

fun distribute_data(
    partitioner: &Partitioner,
    data: Vec<(&str, &str)>
) -> Result<(), String> {
    for (key, value) in data {
        let shard = partitioner.get_shard(key)
        println!("Key {} → Shard {}", key, shard)
    }
    Ok(())
}
```

---

## Step 6: Load Balancing

### Round-robin distribution

```aether
fun create_load_balancer() -> LoadBalancer {
    let mut lb = LoadBalancer::new()
        .with_strategy(LBStrategy::RoundRobin)
    
    for i in 0..5 {
        lb.add_node(
            &format!("backend{}", i),
            &format!("127.0.0.1:{}", 8000 + i as u16)
        ).unwrap()
    }
    
    lb
}

fun balance_requests(
    lb: &mut LoadBalancer,
    requests: Vec<Request>
) -> Result<(), String> {
    for req in requests {
        match lb.next_node() {
            Some(node) => {
                println!("Routing to: {}", node.id)
                lb.forward_request(&req, node)?
            },
            None => return Err("No nodes available".to_string()),
        }
    }
    Ok(())
}
```

---

## Step 7: Distributed Transactions

### Two-phase commit

```aether
fun execute_transaction(
    cluster: &Cluster,
    operations: Vec<Operation>
) -> Result<(), String> {
    let tx = DistributedTransaction::new()
    
    // Add operations
    let tx = operations.iter().fold(
        tx,
        |t, op| t.add_operation(op.clone())
    )
    
    // Phase 1: Prepare
    println!("Phase 1: Preparing transaction...")
    if !cluster.prepare(&tx)? {
        println!("Prepare failed, rolling back...")
        cluster.rollback(&tx)?
        return Err("Transaction failed".to_string())
    }
    
    // Phase 2: Commit
    println!("Phase 2: Committing transaction...")
    cluster.commit(&tx)?
    
    println!("Transaction committed successfully")
    Ok(())
}
```

---

## Step 8: Service Discovery

### Register services

```aether
fun register_service(
    registry: &mut ServiceRegistry,
    name: &str,
    address: &str
) -> Result<(), String> {
    let service = Service::new(name)
        .with_address(address)
        .with_tags(vec!["prod", "v1"])
        .with_health_check(HealthCheck::Http {
            url: format!("{}/health", address),
            interval_secs: 10,
        })
    
    registry.register(service)?
    println!("Service registered: {}", name)
    
    Ok(())
}

fun discover_service(
    registry: &ServiceRegistry,
    name: &str
) -> Result<Vec<Service>, String> {
    registry.discover(name)
        .map_err(|e| format!("Discovery failed: {:?}", e))
}
```

---

## Step 9: Monitoring

### Monitor system health

```aether
fun monitor_system(cluster: &Cluster) -> Result<(), String> {
    loop {
        println!("\n=== Cluster Monitor ===")
        
        // Check leader
        match cluster.get_leader() {
            Some(leader) => println!("Leader: {}", leader),
            None => println!("⚠️ No leader!"),
        }
        
        // Check members
        let members = cluster.get_replicas()
        println!("Members: {}/{}", members.len(), 5)
        
        // Check replication
        for member in &members {
            // Check member health
        }
        
        std::thread::sleep(Duration::from_secs(5))
    }
}
```

---

## Step 10: Complete Distributed System

### Full implementation

```aether
fun main() -> Result<(), String> {
    println!("=== Distributed Database ===\n")
    
    // Step 1: Create cluster
    println!("1. Creating cluster...")
    let mut cluster = create_cluster()?
    
    // Step 2: Initialize Raft
    println!("2. Initializing Raft consensus...")
    initialize_raft(&mut cluster)?
    
    // Step 3: Create storage
    println!("3. Creating distributed storage...")
    let store = create_store(Arc::new(cluster.clone()))
    
    // Step 4: Setup monitoring
    println!("4. Setting up monitoring...")
    setup_failure_handling(&mut cluster)
    
    // Step 5: Write data
    println!("5. Writing data...")
    write_data(&store, "user:1", "Alice")?
    write_data(&store, "user:2", "Bob")?
    
    // Step 6: Read data
    println!("6. Reading data...")
    let user = read_data(&store, "user:1")?
    println!("User: {}", user)
    
    // Step 7: Range scan
    println!("7. Scanning range...")
    let users = range_scan(&store, "user:*")?
    for (key, value) in users {
        println!("{}: {}", key, value)
    }
    
    // Step 8: Monitor
    println!("8. Starting monitor (Ctrl+C to stop)...")
    monitor_system(&cluster)?
    
    Ok(())
}
```

---

## Testing Checklist

- [ ] Cluster creates with 5 nodes
- [ ] Leader elected within 2 seconds
- [ ] Data writes replicated to 3 nodes
- [ ] Data reads return correct values
- [ ] Range scans work correctly
- [ ] Single node failure is handled
- [ ] Automatic failover works
- [ ] Service discovery finds services
- [ ] Transactions commit successfully
- [ ] Monitoring reports health

---

## Exercises

### 1. Add Persistence
Save cluster state to disk for recovery

### 2. Implement Sharding
Distribute data across cluster with Partitioner

### 3. Add Load Balancing
Balance requests across backend nodes

### 4. Implement Snapshots
Create snapshots for fast recovery

### 5. Add Metrics
Export Prometheus-style metrics

---

## Production Deployment

### Configuration

```aether
fun production_config() -> Cluster {
    Cluster::new()
        .with_min_replicas(5)  // 5 replicas for HA
        .with_fault_tolerance(FaultTolerance::Raft)
        // Additional config
}
```

### Monitoring

- Monitor leader health
- Track replication lag
- Alert on member failures
- Track latency percentiles

### Scaling

- Add nodes dynamically
- Rebalance shards
- Monitor resource usage
- Scale read replicas

---

## Next Steps

- Deploy using [DEPLOYMENT.md](DEPLOYMENT.md)
- Monitor using [OPERATIONS.md](OPERATIONS.md)
- Troubleshoot with [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- Read [API_AETHER.md](API_AETHER.md) for advanced features

---

**Congratulations!** You've built a distributed database cluster. From here, scale it and deploy to production.
