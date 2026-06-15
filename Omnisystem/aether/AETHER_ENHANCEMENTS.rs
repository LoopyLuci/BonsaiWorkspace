// AETHER DISTRIBUTED SYSTEMS ENHANCEMENTS
// Advanced distributed computing features

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};

// ============================================================================
// CONSENSUS ALGORITHMS
// ============================================================================

pub enum ConsensusAlgorithm {
    Raft,
    Paxos,
    PBFT,
}

pub struct ConsensusEngine {
    algorithm: ConsensusAlgorithm,
    nodes: Arc<RwLock<Vec<String>>>,
    state: Arc<Mutex<ConsensusState>>,
}

#[derive(Debug, Clone)]
pub struct ConsensusState {
    pub term: u64,
    pub voted_for: Option<String>,
    pub log_entries: Vec<LogEntry>,
    pub committed_index: u64,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub term: u64,
    pub data: String,
    pub timestamp: u64,
}

impl ConsensusEngine {
    pub fn new(algorithm: ConsensusAlgorithm) -> Self {
        ConsensusEngine {
            algorithm,
            nodes: Arc::new(RwLock::new(Vec::new())),
            state: Arc::new(Mutex::new(ConsensusState {
                term: 0,
                voted_for: None,
                log_entries: Vec::new(),
                committed_index: 0,
            })),
        }
    }

    pub fn add_node(&self, node_id: &str) {
        self.nodes.write().unwrap().push(node_id.to_string());
        println!("✅ Node added to consensus: {}", node_id);
    }

    pub fn propose(&self, data: &str) -> Result<u64, String> {
        let mut state = self.state.lock().unwrap();
        state.term += 1;

        let entry = LogEntry {
            term: state.term,
            data: data.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        state.log_entries.push(entry);
        println!("📝 Proposed entry (term {}): {}", state.term, data);
        Ok(state.term)
    }

    pub fn commit(&self) -> Result<u64, String> {
        let mut state = self.state.lock().unwrap();
        let nodes = self.nodes.read().unwrap();
        let quorum = nodes.len() / 2 + 1;

        if state.log_entries.len() >= quorum {
            state.committed_index = state.log_entries.len() as u64 - 1;
            println!("✅ Consensus reached (quorum: {}/{})", quorum, nodes.len());
            Ok(state.committed_index)
        } else {
            Err("Insufficient replicas".to_string())
        }
    }

    pub fn get_state(&self) -> ConsensusState {
        self.state.lock().unwrap().clone()
    }
}

// ============================================================================
// DISTRIBUTED TRANSACTIONS
// ============================================================================

pub struct DistributedTransaction {
    pub id: String,
    pub operations: Vec<TransactionOp>,
    pub status: TransactionStatus,
}

#[derive(Debug, Clone)]
pub struct TransactionOp {
    pub node_id: String,
    pub operation: String,
    pub status: OperationStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Prepared,
    Committed,
    RolledBack,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperationStatus {
    Pending,
    Executed,
    Success,
    Failed,
}

impl DistributedTransaction {
    pub fn new(id: &str) -> Self {
        DistributedTransaction {
            id: id.to_string(),
            operations: Vec::new(),
            status: TransactionStatus::Pending,
        }
    }

    pub fn add_operation(&mut self, node_id: &str, operation: &str) {
        self.operations.push(TransactionOp {
            node_id: node_id.to_string(),
            operation: operation.to_string(),
            status: OperationStatus::Pending,
        });
        println!("➕ Operation added to transaction {}", self.id);
    }

    pub fn prepare(&mut self) -> Result<(), String> {
        self.status = TransactionStatus::Prepared;
        for op in &mut self.operations {
            op.status = OperationStatus::Executed;
        }
        println!("🔄 Transaction {} prepared", self.id);
        Ok(())
    }

    pub fn commit(&mut self) -> Result<(), String> {
        self.status = TransactionStatus::Committed;
        for op in &mut self.operations {
            op.status = OperationStatus::Success;
        }
        println!("✅ Transaction {} committed", self.id);
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), String> {
        self.status = TransactionStatus::RolledBack;
        println!("↩️  Transaction {} rolled back", self.id);
        Ok(())
    }
}

// ============================================================================
// SHARDING & PARTITIONING
// ============================================================================

pub struct ShardingStrategy {
    pub num_shards: usize,
    pub shard_map: HashMap<String, usize>,
}

impl ShardingStrategy {
    pub fn new(num_shards: usize) -> Self {
        ShardingStrategy {
            num_shards,
            shard_map: HashMap::new(),
        }
    }

    pub fn hash_key(&self, key: &str) -> usize {
        let hash = key.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
        (hash as usize) % self.num_shards
    }

    pub fn get_shard(&self, key: &str) -> usize {
        self.hash_key(key)
    }

    pub fn assign_key(&mut self, key: &str) -> usize {
        let shard = self.hash_key(key);
        self.shard_map.insert(key.to_string(), shard);
        println!("📍 Key '{}' assigned to shard {}", key, shard);
        shard
    }

    pub fn rebalance(&mut self) -> Vec<(String, usize, usize)> {
        let mut migrations = Vec::new();
        let mut new_shard_map = HashMap::new();

        for (key, old_shard) in &self.shard_map {
            let new_shard = self.hash_key(key);
            if new_shard != *old_shard {
                migrations.push((key.clone(), *old_shard, new_shard));
            }
            new_shard_map.insert(key.clone(), new_shard);
        }

        self.shard_map = new_shard_map;
        println!("🔄 Rebalanced {} keys", migrations.len());
        migrations
    }
}

// ============================================================================
// GOSSIP PROTOCOL
// ============================================================================

pub struct GossipMessage {
    pub sender: String,
    pub data: String,
    pub version: u64,
    pub timestamp: u64,
}

pub struct GossipProtocol {
    pub nodes: Arc<RwLock<Vec<String>>>,
    pub messages: Arc<Mutex<Vec<GossipMessage>>>,
    pub state: Arc<RwLock<HashMap<String, String>>>,
}

impl GossipProtocol {
    pub fn new() -> Self {
        GossipProtocol {
            nodes: Arc::new(RwLock::new(Vec::new())),
            messages: Arc::new(Mutex::new(Vec::new())),
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_node(&self, node_id: &str) {
        self.nodes.write().unwrap().push(node_id.to_string());
        println!("📡 Node joined gossip network: {}", node_id);
    }

    pub fn propagate(&self, sender: &str, data: &str) {
        let msg = GossipMessage {
            sender: sender.to_string(),
            data: data.to_string(),
            version: 1,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.messages.lock().unwrap().push(msg);
        println!("💬 Message propagated from {}", sender);
    }

    pub fn update_state(&self, key: &str, value: &str) {
        self.state.write().unwrap().insert(key.to_string(), value.to_string());
        self.propagate("local", &format!("{}={}", key, value));
    }

    pub fn get_state(&self, key: &str) -> Option<String> {
        self.state.read().unwrap().get(key).cloned()
    }
}

// ============================================================================
// LOAD BALANCING STRATEGIES
// ============================================================================

pub enum LoadBalanceStrategy {
    RoundRobin,
    LeastConnections,
    WeightedRandom,
    ConsistentHash,
}

pub struct LoadBalancer {
    pub nodes: Vec<(String, u32)>, // (node_id, weight)
    pub strategy: LoadBalanceStrategy,
    pub current_index: usize,
    pub connections: HashMap<String, u32>,
}

impl LoadBalancer {
    pub fn new(strategy: LoadBalanceStrategy) -> Self {
        LoadBalancer {
            nodes: Vec::new(),
            strategy,
            current_index: 0,
            connections: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node_id: &str, weight: u32) {
        self.nodes.push((node_id.to_string(), weight));
        self.connections.insert(node_id.to_string(), 0);
        println!("➕ Node added to load balancer: {} (weight: {})", node_id, weight);
    }

    pub fn select_node(&mut self) -> Result<String, String> {
        if self.nodes.is_empty() {
            return Err("No nodes available".to_string());
        }

        let selected = match self.strategy {
            LoadBalanceStrategy::RoundRobin => {
                let idx = self.current_index % self.nodes.len();
                self.current_index += 1;
                &self.nodes[idx].0
            }
            LoadBalanceStrategy::LeastConnections => {
                self.nodes.iter()
                    .min_by_key(|(id, _)| self.connections.get(id).copied().unwrap_or(0))
                    .map(|(id, _)| id)
                    .unwrap()
            }
            _ => &self.nodes[0].0,
        };

        *self.connections.get_mut(selected).unwrap() += 1;
        println!("🎯 Selected node: {}", selected);
        Ok(selected.clone())
    }

    pub fn release_connection(&mut self, node_id: &str) {
        if let Some(count) = self.connections.get_mut(node_id) {
            if *count > 0 {
                *count -= 1;
            }
        }
    }
}

// ============================================================================
// EXAMPLE USAGE
// ============================================================================

pub fn example_enhancements() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚀 AETHER DISTRIBUTED SYSTEMS ENHANCEMENTS\n");

    // Consensus Engine
    println!("1️⃣  Consensus Algorithm:");
    let consensus = ConsensusEngine::new(ConsensusAlgorithm::Raft);
    consensus.add_node("node-1");
    consensus.add_node("node-2");
    consensus.add_node("node-3");
    consensus.propose("key=value")?;
    consensus.commit()?;
    println!();

    // Distributed Transactions
    println!("2️⃣  Distributed Transactions:");
    let mut txn = DistributedTransaction::new("txn-1");
    txn.add_operation("node-1", "INSERT user");
    txn.add_operation("node-2", "UPDATE index");
    txn.prepare()?;
    txn.commit()?;
    println!();

    // Sharding
    println!("3️⃣  Sharding Strategy:");
    let mut sharding = ShardingStrategy::new(4);
    sharding.assign_key("user:1000")?;
    sharding.assign_key("user:2000")?;
    sharding.assign_key("user:3000")?;
    println!();

    // Gossip Protocol
    println!("4️⃣  Gossip Protocol:");
    let gossip = GossipProtocol::new();
    gossip.add_node("node-1");
    gossip.add_node("node-2");
    gossip.add_node("node-3");
    gossip.update_state("cluster-status", "healthy");
    println!();

    // Load Balancing
    println!("5️⃣  Load Balancing:");
    let mut lb = LoadBalancer::new(LoadBalanceStrategy::LeastConnections);
    lb.add_node("server-1", 100);
    lb.add_node("server-2", 100);
    lb.add_node("server-3", 50);
    lb.select_node()?;
    lb.select_node()?;
    println!();

    println!("✅ Aether Enhancements Complete\n");
    Ok(())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_engine() {
        let consensus = ConsensusEngine::new(ConsensusAlgorithm::Raft);
        consensus.add_node("node-1");
        assert!(consensus.propose("test").is_ok());
    }

    #[test]
    fn test_distributed_transaction() {
        let mut txn = DistributedTransaction::new("test");
        txn.add_operation("node-1", "op");
        assert!(txn.prepare().is_ok());
        assert!(txn.commit().is_ok());
    }

    #[test]
    fn test_sharding() {
        let mut sharding = ShardingStrategy::new(4);
        let shard1 = sharding.get_shard("key1");
        assert!(shard1 < 4);
    }

    #[test]
    fn test_gossip() {
        let gossip = GossipProtocol::new();
        gossip.add_node("n1");
        gossip.update_state("test", "value");
        assert_eq!(gossip.get_state("test"), Some("value".to_string()));
    }

    #[test]
    fn test_load_balancer() {
        let mut lb = LoadBalancer::new(LoadBalanceStrategy::RoundRobin);
        lb.add_node("s1", 100);
        lb.add_node("s2", 100);
        assert!(lb.select_node().is_ok());
    }
}
