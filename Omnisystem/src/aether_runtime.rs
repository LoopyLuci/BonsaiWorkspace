// AETHER RUNTIME - Distributed computing and networking
// Multi-node coordination, message passing, distributed algorithms
// Version: 2.0

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

/// Node identifier in distributed system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

/// Network address
#[derive(Debug, Clone)]
pub struct NetworkAddress {
    pub host: String,
    pub port: u16,
}

impl NetworkAddress {
    pub fn new(host: &str, port: u16) -> Self {
        NetworkAddress {
            host: host.to_string(),
            port,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// Message for inter-node communication
#[derive(Debug, Clone)]
pub struct Message {
    pub from: NodeId,
    pub to: NodeId,
    pub message_type: MessageType,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum MessageType {
    Data,
    Request,
    Response,
    Heartbeat,
    Sync,
    Control,
}

/// Distributed Task
#[derive(Debug, Clone)]
pub struct DistributedTask {
    pub task_id: String,
    pub name: String,
    pub status: TaskStatus,
    pub assigned_node: Option<NodeId>,
    pub progress: f32,
    pub result: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Distributed System Node
pub struct Node {
    pub id: NodeId,
    pub address: NetworkAddress,
    pub status: Arc<Mutex<NodeStatus>>,
    pub inbox: Arc<Mutex<VecDeque<Message>>>,
    pub outbox: Arc<Mutex<VecDeque<Message>>>,
    pub state: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

#[derive(Debug, Clone)]
pub enum NodeStatus {
    Alive,
    Suspected,
    Dead,
}

impl Node {
    pub fn new(id: NodeId, address: NetworkAddress) -> Self {
        Node {
            id,
            address,
            status: Arc::new(Mutex::new(NodeStatus::Alive)),
            inbox: Arc::new(Mutex::new(VecDeque::new())),
            outbox: Arc::new(Mutex::new(VecDeque::new())),
            state: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn send_message(&self, message: Message) -> Result<(), DistributedError> {
        let mut outbox = self.outbox.lock().unwrap();
        outbox.push_back(message);
        Ok(())
    }

    pub fn receive_message(&self) -> Option<Message> {
        let mut inbox = self.inbox.lock().unwrap();
        inbox.pop_front()
    }

    pub fn set_state(&self, key: String, value: Vec<u8>) {
        let mut state = self.state.lock().unwrap();
        state.insert(key, value);
    }

    pub fn get_state(&self, key: &str) -> Option<Vec<u8>> {
        let state = self.state.lock().unwrap();
        state.get(key).cloned()
    }
}

/// Distributed Cluster
pub struct Cluster {
    pub nodes: HashMap<NodeId, Arc<Node>>,
    pub task_queue: Arc<Mutex<VecDeque<DistributedTask>>>,
    pub consensus_algorithm: ConsensusAlgorithm,
}

#[derive(Debug, Clone)]
pub enum ConsensusAlgorithm {
    Raft,
    Paxos,
    BFT,
}

impl Cluster {
    pub fn new(consensus: ConsensusAlgorithm) -> Self {
        Cluster {
            nodes: HashMap::new(),
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            consensus_algorithm: consensus,
        }
    }

    pub fn add_node(&mut self, node: Arc<Node>) -> Result<(), DistributedError> {
        self.nodes.insert(node.id, node);
        Ok(())
    }

    pub fn remove_node(&mut self, node_id: NodeId) -> Result<(), DistributedError> {
        self.nodes.remove(&node_id)
            .ok_or(DistributedError::NodeNotFound)?;
        Ok(())
    }

    pub fn submit_task(&self, task: DistributedTask) -> Result<String, DistributedError> {
        let mut queue = self.task_queue.lock().unwrap();
        queue.push_back(task.clone());
        Ok(task.task_id)
    }

    pub fn get_task(&self, task_id: &str) -> Result<DistributedTask, DistributedError> {
        let queue = self.task_queue.lock().unwrap();
        queue.iter()
            .find(|t| t.task_id == task_id)
            .cloned()
            .ok_or(DistributedError::TaskNotFound)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn alive_nodes(&self) -> usize {
        self.nodes.values()
            .filter(|node| {
                if let Ok(status) = node.status.lock() {
                    matches!(*status, NodeStatus::Alive)
                } else {
                    false
                }
            })
            .count()
    }
}

/// Raft Consensus Implementation
pub struct RaftNode {
    pub node_id: NodeId,
    pub state: Arc<Mutex<RaftState>>,
    pub log: Arc<Mutex<Vec<LogEntry>>>,
    pub voted_for: Arc<Mutex<Option<NodeId>>>,
}

#[derive(Debug, Clone, Copy)]
pub enum RaftState {
    Follower,
    Candidate,
    Leader,
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub term: u64,
    pub data: Vec<u8>,
}

impl RaftNode {
    pub fn new(node_id: NodeId) -> Self {
        RaftNode {
            node_id,
            state: Arc::new(Mutex::new(RaftState::Follower)),
            log: Arc::new(Mutex::new(Vec::new())),
            voted_for: Arc::new(Mutex::new(None)),
        }
    }

    pub fn append_entry(&self, entry: LogEntry) -> Result<(), DistributedError> {
        let mut log = self.log.lock().unwrap();
        log.push(entry);
        Ok(())
    }

    pub fn get_log_entries(&self) -> Result<Vec<LogEntry>, DistributedError> {
        let log = self.log.lock().unwrap();
        Ok(log.clone())
    }

    pub fn commit_entry(&self, index: usize) -> Result<(), DistributedError> {
        let log = self.log.lock().unwrap();
        if index < log.len() {
            Ok(())
        } else {
            Err(DistributedError::InvalidIndex)
        }
    }

    pub fn become_leader(&self) -> Result<(), DistributedError> {
        let mut state = self.state.lock().unwrap();
        *state = RaftState::Leader;
        Ok(())
    }

    pub fn become_follower(&self) -> Result<(), DistributedError> {
        let mut state = self.state.lock().unwrap();
        *state = RaftState::Follower;
        Ok(())
    }

    pub fn vote_for(&self, candidate: NodeId) -> Result<bool, DistributedError> {
        let mut voted = self.voted_for.lock().unwrap();
        if voted.is_none() {
            *voted = Some(candidate);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Data Partitioning (Sharding)
pub struct Partitioner {
    pub partitions: Vec<Vec<u8>>,
    pub shard_count: usize,
}

impl Partitioner {
    pub fn new(shard_count: usize) -> Self {
        Partitioner {
            partitions: vec![Vec::new(); shard_count],
            shard_count,
        }
    }

    pub fn hash_key(&self, key: &str) -> usize {
        let mut hash = 0u32;
        for byte in key.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }
        (hash as usize) % self.shard_count
    }

    pub fn put(&mut self, key: &str, value: Vec<u8>) {
        let partition = self.hash_key(key);
        self.partitions[partition].extend_from_slice(&value);
    }

    pub fn get_partition(&self, partition_id: usize) -> Option<&Vec<u8>> {
        self.partitions.get(partition_id)
    }
}

/// Load Balancer
pub struct LoadBalancer {
    pub nodes: Vec<NodeId>,
    pub current_index: Arc<Mutex<usize>>,
}

impl LoadBalancer {
    pub fn new(nodes: Vec<NodeId>) -> Self {
        LoadBalancer {
            nodes,
            current_index: Arc::new(Mutex::new(0)),
        }
    }

    pub fn get_next_node(&self) -> Option<NodeId> {
        if self.nodes.is_empty() {
            return None;
        }

        let mut idx = self.current_index.lock().unwrap();
        let node = self.nodes[*idx];
        *idx = (*idx + 1) % self.nodes.len();
        Some(node)
    }

    pub fn add_node(&mut self, node_id: NodeId) {
        self.nodes.push(node_id);
    }

    pub fn remove_node(&mut self, node_id: NodeId) {
        self.nodes.retain(|&n| n != node_id);
    }
}

/// Distributed Cache with Replication
pub struct DistributedCache {
    pub cache: Arc<Mutex<HashMap<String, CacheValue>>>,
    pub replicas: usize,
}

#[derive(Debug, Clone)]
pub struct CacheValue {
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub replicated_on: Vec<NodeId>,
}

impl DistributedCache {
    pub fn new(replicas: usize) -> Self {
        DistributedCache {
            cache: Arc::new(Mutex::new(HashMap::new())),
            replicas,
        }
    }

    pub fn set(&self, key: String, value: Vec<u8>, replicated_on: Vec<NodeId>) {
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key, CacheValue {
            data: value,
            timestamp: Self::current_time(),
            replicated_on,
        });
    }

    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        let cache = self.cache.lock().unwrap();
        cache.get(key).map(|v| v.data.clone())
    }

    pub fn delete(&self, key: &str) {
        let mut cache = self.cache.lock().unwrap();
        cache.remove(key);
    }

    pub fn size(&self) -> usize {
        let cache = self.cache.lock().unwrap();
        cache.len()
    }

    fn current_time() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

/// Distributed Errors
#[derive(Debug)]
pub enum DistributedError {
    NodeNotFound,
    TaskNotFound,
    InvalidIndex,
    MessageFailed(String),
    ConsensusFailure,
    ReplicationFailed,
}

impl std::fmt::Display for DistributedError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DistributedError::NodeNotFound => write!(f, "Node not found"),
            DistributedError::TaskNotFound => write!(f, "Task not found"),
            DistributedError::InvalidIndex => write!(f, "Invalid index"),
            DistributedError::MessageFailed(msg) => write!(f, "Message failed: {}", msg),
            DistributedError::ConsensusFailure => write!(f, "Consensus failure"),
            DistributedError::ReplicationFailed => write!(f, "Replication failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_address() {
        let addr = NetworkAddress::new("localhost", 8080);
        assert_eq!(addr.to_string(), "localhost:8080");
    }

    #[test]
    fn test_node_creation() {
        let node = Node::new(NodeId(1), NetworkAddress::new("localhost", 8080));
        assert_eq!(node.id, NodeId(1));
    }

    #[test]
    fn test_cluster_operations() {
        let mut cluster = Cluster::new(ConsensusAlgorithm::Raft);
        let node = Arc::new(Node::new(NodeId(1), NetworkAddress::new("localhost", 8080)));

        assert!(cluster.add_node(node).is_ok());
        assert_eq!(cluster.node_count(), 1);
    }

    #[test]
    fn test_raft_node() {
        let raft = RaftNode::new(NodeId(1));
        let entry = LogEntry {
            term: 1,
            data: b"data".to_vec(),
        };

        assert!(raft.append_entry(entry).is_ok());
        assert!(raft.become_leader().is_ok());
    }

    #[test]
    fn test_partitioner() {
        let mut partitioner = Partitioner::new(4);
        partitioner.put("key1", b"value1".to_vec());

        let partition = partitioner.hash_key("key1");
        assert!(partition < 4);
    }

    #[test]
    fn test_load_balancer() {
        let nodes = vec![NodeId(1), NodeId(2), NodeId(3)];
        let lb = LoadBalancer::new(nodes);

        let first = lb.get_next_node();
        assert_eq!(first, Some(NodeId(1)));
    }

    #[test]
    fn test_distributed_cache() {
        let cache = DistributedCache::new(2);
        cache.set("key1".to_string(), b"value1".to_vec(), vec![NodeId(1), NodeId(2)]);

        let value = cache.get("key1");
        assert_eq!(value, Some(b"value1".to_vec()));
    }
}
