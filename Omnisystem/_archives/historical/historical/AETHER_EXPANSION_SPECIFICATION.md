# AETHER: Distributed Systems Language - Complete Specification

**Aether** is the distributed application layer. It must be capable of everything Go, Erlang, Scala, Akka, and Kafka can do—but with superior resilience, correctness, and visibility.

---

## I. CORE AETHER CAPABILITIES (Already Implemented)

✅ Actor model (isolated processes, message passing)  
✅ Location transparency (local/remote same code)  
✅ Supervision trees (automatic failure recovery)  
✅ Message-typed communication  
✅ CRDT support (eventual consistency)  
✅ Event sourcing  
✅ Effect system inheritance from Titan  

---

## II. MISSING CAPABILITIES THAT MUST BE ADDED

### A. CONSENSUS & STATE REPLICATION

#### A.1 Raft Consensus
```aether
actor RaftNode {
    var term: i64 = 0;
    var voted_for: Option<String> = None;
    var log: Vec<LogEntry> = Vec::new();
    var commit_index: i64 = 0;
    var last_applied: i64 = 0;
    
    // Leader state
    var next_index: Map<String, i64> = Map::new();
    var match_index: Map<String, i64> = Map::new();
    
    handle Heartbeat(leader_term: i64, leader_id: String, leader_commit: i64) -> HeartbeatResponse ! {io} {
        if leader_term >= self.term {
            self.term = leader_term;
            self.voted_for = Some(leader_id);
            self.commit_index = min(leader_commit, self.log.len() as i64);
            return HeartbeatResponse { term: self.term, success: true };
        }
        return HeartbeatResponse { term: self.term, success: false };
    }
    
    handle AppendEntries(
        leader_term: i64,
        leader_id: String,
        prev_log_index: i64,
        prev_log_term: i64,
        entries: Vec<LogEntry>,
        leader_commit: i64
    ) -> AppendEntriesResponse ! {io, alloc} {
        // Raft append entries protocol
        if leader_term >= self.term {
            self.term = leader_term;
            self.voted_for = Some(leader_id);
            
            // Check if we have prev_log_index
            if prev_log_index >= 0 && prev_log_index < self.log.len() as i64 {
                if self.log[prev_log_index as usize].term != prev_log_term {
                    return AppendEntriesResponse { term: self.term, success: false };
                }
            }
            
            // Append entries
            for entry in entries {
                self.log.push(entry);
            }
            
            // Update commit index
            self.commit_index = min(leader_commit, self.log.len() as i64);
            
            return AppendEntriesResponse { term: self.term, success: true };
        }
        return AppendEntriesResponse { term: self.term, success: false };
    }
    
    handle RequestVote(
        candidate_term: i64,
        candidate_id: String,
        last_log_index: i64,
        last_log_term: i64
    ) -> RequestVoteResponse ! {io} {
        if candidate_term >= self.term {
            self.term = candidate_term;
            
            if self.voted_for == None || self.voted_for == Some(candidate_id) {
                // Check if candidate's log is up-to-date
                if last_log_term >= (if self.log.len() > 0 { self.log[self.log.len()-1].term } else { 0 }) {
                    if last_log_index >= self.log.len() as i64 - 1 {
                        self.voted_for = Some(candidate_id);
                        return RequestVoteResponse { term: self.term, vote_granted: true };
                    }
                }
            }
        }
        return RequestVoteResponse { term: self.term, vote_granted: false };
    }
    
    on_timeout() ! {io} {
        // Handle election timeout
        if self.role == Role::Follower {
            become_candidate();
        } else if self.role == Role::Candidate {
            start_new_election();
        }
    }
}
```

**Requirements**:
- [ ] Full Raft implementation (leader election, log replication, safety)
- [ ] Term-based voting and log matching
- [ ] Election timeout handling
- [ ] Heartbeat mechanism
- [ ] Log compaction/snapshotting
- [ ] Configuration changes

#### A.2 Paxos Variants
```aether
actor PaxosAcceptor {
    var promised_proposal: i64 = 0;
    var accepted_proposal: i64 = 0;
    var accepted_value: Any = None;
    
    handle Prepare(proposal_id: i64) -> PrepareResponse ! {io} {
        if proposal_id > self.promised_proposal {
            self.promised_proposal = proposal_id;
            return PrepareResponse { 
                ok: true, 
                promised: proposal_id,
                accepted_proposal: self.accepted_proposal,
                accepted_value: self.accepted_value 
            };
        }
        return PrepareResponse { ok: false, promised: self.promised_proposal };
    }
    
    handle Accept(proposal_id: i64, value: Any) -> AcceptResponse ! {io} {
        if proposal_id >= self.promised_proposal {
            self.accepted_proposal = proposal_id;
            self.accepted_value = value;
            return AcceptResponse { ok: true, proposal_id: proposal_id };
        }
        return AcceptResponse { ok: false, proposal_id: self.promised_proposal };
    }
}

actor PaxosProposer {
    var proposal_id: i64 = 0;
    var value: Any = None;
    var quorum_size: i64 = 3;
    
    async fn propose(value: Any) -> Result<Any, PaxosError> ! {io, alloc} {
        self.proposal_id = self.proposal_id + 1;
        self.value = value;
        
        // Phase 1: Prepare
        let acceptors = get_acceptor_nodes();
        let mut prepared = 0;
        let mut max_accepted_proposal = 0;
        let mut max_accepted_value = None;
        
        for acceptor in acceptors {
            let response = await acceptor.ask(Prepare { proposal_id: self.proposal_id });
            if response.ok {
                prepared = prepared + 1;
                if response.accepted_proposal > max_accepted_proposal {
                    max_accepted_proposal = response.accepted_proposal;
                    max_accepted_value = response.accepted_value;
                }
            }
        }
        
        if prepared < self.quorum_size {
            return Err(PaxosError::NotAccepted);
        }
        
        // Phase 2: Accept
        let final_value = if max_accepted_value != None { 
            max_accepted_value 
        } else { 
            self.value 
        };
        
        let mut accepted = 0;
        for acceptor in acceptors {
            let response = await acceptor.ask(Accept { 
                proposal_id: self.proposal_id, 
                value: final_value 
            });
            if response.ok {
                accepted = accepted + 1;
            }
        }
        
        if accepted < self.quorum_size {
            return Err(PaxosError::NotAccepted);
        }
        
        return Ok(final_value);
    }
}
```

**Requirements**:
- [ ] Classic Paxos (prepare, promise, accept, learn phases)
- [ ] Multi-Paxos for sequence of values
- [ ] Fast Paxos variant
- [ ] Byzantine Paxos (optional for extra resilience)
- [ ] Quorum calculation

#### A.3 CRDT Integration
```aether
pub struct CounterCRDT {
    vec: Map<ActorId, i64>,
}

impl CounterCRDT {
    pub fn increment(&mut self, actor_id: ActorId) {
        let val = self.vec.get(actor_id).unwrap_or(0);
        self.vec.insert(actor_id, val + 1);
    }
    
    pub fn value(&self) -> i64 {
        return self.vec.iter().map(|(_, v)| v).sum();
    }
    
    pub fn merge(&mut self, other: &CounterCRDT) {
        for (actor_id, val) in other.vec.iter() {
            let my_val = self.vec.get(actor_id).unwrap_or(0);
            self.vec.insert(actor_id, max(my_val, val));
        }
    }
}

pub struct SetCRDT<T> {
    added: Set<T>,
    removed: Set<T>,
}

impl<T> SetCRDT<T> {
    pub fn add(&mut self, item: T) {
        self.added.insert(item);
    }
    
    pub fn remove(&mut self, item: T) {
        self.removed.insert(item);
    }
    
    pub fn contains(&self, item: &T) -> bool {
        return self.added.contains(item) && !self.removed.contains(item);
    }
    
    pub fn merge(&mut self, other: &SetCRDT<T>) {
        self.added.merge(&other.added);
        self.removed.merge(&other.removed);
    }
}
```

**Requirements**:
- [ ] Conflict-free replicated data types
- [ ] Counter CRDT (increment-only)
- [ ] Set CRDT (add-remove-idempotent)
- [ ] Map CRDT
- [ ] Sequence CRDT (RGA)
- [ ] Automatic convergence guarantees
- [ ] Composable CRDTs

---

### B. DISTRIBUTED COORDINATION

#### B.1 Time-Based Message Scheduling
```aether
actor SchedulerActor {
    var scheduled_messages: PriorityQueue<(Time, ActorRef, Message)> = PriorityQueue::new();
    
    handle Schedule(time: Time, target: ActorRef, message: Message) ! {alloc} {
        self.scheduled_messages.push((time, target, message));
    }
    
    on_tick() ! {io} {
        let now = current_time();
        while let Some((time, target, message)) = self.scheduled_messages.peek() {
            if time <= now {
                self.scheduled_messages.pop();
                target.send(message);
            } else {
                break;
            }
        }
    }
}

pub fn schedule_after(actor: ActorRef, delay: Duration, message: Message) ! {io} {
    let time = current_time() + delay;
    let scheduler = get_scheduler();
    scheduler.send(Schedule { time: time, target: actor, message: message });
}

pub fn schedule_at(actor: ActorRef, time: Time, message: Message) ! {io} {
    let scheduler = get_scheduler();
    scheduler.send(Schedule { time: time, target: actor, message: message });
}
```

**Requirements**:
- [ ] Global scheduler actor
- [ ] Priority queue for scheduled messages
- [ ] Time-based sorting
- [ ] Cancellable timers
- [ ] Recurring messages
- [ ] Timeout enforcement

#### B.2 Distributed Tracing
```aether
pub struct TraceContext {
    trace_id: u64,
    span_id: u64,
    parent_span_id: u64,
}

pub fn current_trace() -> TraceContext ! {io} {
    return THREAD_LOCAL_TRACE.clone();
}

pub fn with_trace<T>(trace: TraceContext, f: fn() -> T) -> T ! {io} {
    let old_trace = THREAD_LOCAL_TRACE.clone();
    THREAD_LOCAL_TRACE.set(trace);
    let result = f();
    THREAD_LOCAL_TRACE.set(old_trace);
    return result;
}

actor TracingActor {
    var spans: Map<u64, Span> = Map::new();
    
    handle RecordSpan(span: Span) ! {io} {
        self.spans.insert(span.span_id, span);
        emit_to_jaeger(span);
    }
}

pub fn start_span(name: String) -> u64 ! {io} {
    let trace = current_trace();
    let span_id = generate_span_id();
    let span = Span {
        trace_id: trace.trace_id,
        span_id: span_id,
        parent_span_id: trace.span_id,
        name: name,
        start_time: current_time(),
        end_time: 0,
        tags: Map::new(),
    };
    return span_id;
}

pub fn end_span(span_id: u64) ! {io} {
    let tracer = get_tracer();
    tracer.send(RecordSpan { span_id: span_id });
}

pub fn add_trace_tag(key: String, value: String) ! {io} {
    let mut trace = current_trace();
    trace.tags.insert(key, value);
}
```

**Requirements**:
- [ ] Trace ID propagation across actors
- [ ] Parent-child span relationships
- [ ] Jaeger/Zipkin export
- [ ] W3C Trace Context compatibility
- [ ] Sampling policies
- [ ] Performance overhead <2%

#### B.3 Load Balancing & Sharding
```aether
pub enum ShardingStrategy {
    Range(i64),           // Range-based on key
    Modulo(i64),          // key % num_shards
    Consistent(i64),      // Consistent hashing
    Custom(fn(Any) -> i64),
}

actor ShardRouter {
    var shards: Vec<ActorRef> = Vec::new();
    var strategy: ShardingStrategy = ShardingStrategy::Modulo(32);
    
    handle Route(key: Any, message: Message) ! {io} {
        let shard_id = match self.strategy {
            ShardingStrategy::Range(size) => {
                let key_val = hash(key);
                (key_val / size) % self.shards.len() as i64
            }
            ShardingStrategy::Modulo(num) => {
                hash(key) % num
            }
            ShardingStrategy::Consistent(num) => {
                consistent_hash(key, num)
            }
            ShardingStrategy::Custom(f) => {
                f(key) % self.shards.len() as i64
            }
        };
        
        self.shards[shard_id as usize].send(message);
    }
    
    handle RebalanceShard(old_id: i64, new_id: i64) ! {io} {
        // Handle shard rebalancing when adding/removing nodes
    }
}
```

**Requirements**:
- [ ] Range-based sharding
- [ ] Modulo sharding
- [ ] Consistent hashing
- [ ] Custom sharding functions
- [ ] Dynamic rebalancing
- [ ] Hot-spot detection

---

### C. RESILIENCE & FAULT TOLERANCE

#### C.1 Advanced Supervision Strategies
```aether
pub enum SupervisionStrategy {
    OneForOne,              // Restart only failed child
    OneForAll,              // Restart all children
    RestForOne,             // Restart failed + all after it
    Quarantine,             // Isolate failed child, restart separately
    Exponential,            // Exponential backoff retry
    Circuit(i64, i64),      // Fail-fast after N failures in M seconds
}

actor Supervisor {
    var children: Vec<ActorRef> = Vec::new();
    var strategy: SupervisionStrategy = SupervisionStrategy::OneForOne;
    var restart_counts: Map<ActorRef, i64> = Map::new();
    var max_restarts: i64 = 5;
    var restart_window: Duration = Duration::from_secs(60);
    
    handle ChildFailed(child: ActorRef, reason: String) ! {io} {
        let restart_count = self.restart_counts.get(&child).unwrap_or(0);
        
        if restart_count >= self.max_restarts {
            // Too many restarts, escalate to parent supervisor
            emit_event(ChildDiedPermanently { child: child });
            return;
        }
        
        match self.strategy {
            SupervisionStrategy::OneForOne => {
                restart_child(child);
            }
            SupervisionStrategy::OneForAll => {
                for c in self.children.iter() {
                    restart_child(*c);
                }
            }
            SupervisionStrategy::RestForOne => {
                let idx = self.children.iter().position(|&c| c == child).unwrap();
                for i in idx..self.children.len() {
                    restart_child(self.children[i]);
                }
            }
            SupervisionStrategy::Exponential => {
                let backoff = Duration::from_millis(100 * (1 << restart_count));
                schedule_after(this, backoff, RestartChild { child: child });
            }
            SupervisionStrategy::Circuit(max_failures, window_secs) => {
                if should_open_circuit(child, max_failures, window_secs) {
                    emit_event(CircuitOpen { child: child });
                } else {
                    restart_child(child);
                }
            }
            _ => {}
        }
        
        self.restart_counts.insert(child, restart_count + 1);
    }
    
    handle RestartChild(child: ActorRef) ! {io} {
        restart_child(child);
    }
}
```

**Requirements**:
- [ ] One-for-one, one-for-all, rest-for-one
- [ ] Exponential backoff
- [ ] Circuit breaker pattern
- [ ] Quarantine/isolation
- [ ] Cascading failure detection
- [ ] Escalation to parent supervisor

#### C.2 Deadletter Queues
```aether
actor DeadLetterQueue {
    var messages: Vec<(ActorRef, Message)> = Vec::new();
    
    handle DeadLetter(target: ActorRef, message: Message) ! {alloc} {
        self.messages.push((target, message));
        emit_event(DeadLetterReceived { target: target, message: message });
    }
    
    handle ProcessDeadLetters() ! {io} {
        let to_process = self.messages.clone();
        self.messages.clear();
        
        for (target, message) in to_process {
            if target_is_alive(target) {
                target.send(message);
            } else {
                emit_event(CannotDeliverDeadLetter { target: target });
            }
        }
    }
    
    handle DumpDeadLetters(filename: String) ! {io} {
        serialize_to_file(self.messages, filename);
    }
}

pub fn send_with_deadletter(actor: ActorRef, message: Message) ! {io} {
    if !actor.is_alive() {
        let dlq = get_deadletter_queue();
        dlq.send(DeadLetter { target: actor, message: message });
    } else {
        actor.send(message);
    }
}
```

**Requirements**:
- [ ] Automatic dead-letter capture
- [ ] Dead-letter replay
- [ ] Dead-letter debugging
- [ ] Dead-letter export

---

### D. OBSERVABILITY & MONITORING

#### D.1 Metrics Collection
```aether
actor MetricsCollector {
    var counters: Map<String, i64> = Map::new();
    var gauges: Map<String, f64> = Map::new();
    var histograms: Map<String, Vec<u64>> = Map::new();
    
    handle IncrementCounter(name: String, value: i64) ! {alloc} {
        let current = self.counters.get(&name).unwrap_or(0);
        self.counters.insert(name, current + value);
    }
    
    handle SetGauge(name: String, value: f64) ! {alloc} {
        self.gauges.insert(name, value);
    }
    
    handle RecordHistogram(name: String, value: u64) ! {alloc} {
        let hist = self.histograms.get(&name).unwrap_or(Vec::new());
        hist.push(value);
        self.histograms.insert(name, hist);
    }
    
    handle ExportMetrics() -> MetricsSnapshot ! {alloc} {
        return MetricsSnapshot {
            counters: self.counters.clone(),
            gauges: self.gauges.clone(),
            histograms: self.histograms.clone(),
        };
    }
}

pub fn emit_counter(name: String, value: i64) ! {io} {
    let metrics = get_metrics_collector();
    metrics.send(IncrementCounter { name: name, value: value });
}

pub fn emit_gauge(name: String, value: f64) ! {io} {
    let metrics = get_metrics_collector();
    metrics.send(SetGauge { name: name, value: value });
}

pub fn emit_histogram(name: String, value: u64) ! {io} {
    let metrics = get_metrics_collector();
    metrics.send(RecordHistogram { name: name, value: value });
}
```

**Requirements**:
- [ ] Counter (monotonic increment)
- [ ] Gauge (arbitrary value)
- [ ] Histogram (distribution)
- [ ] Summary (quantiles)
- [ ] Prometheus export format
- [ ] InfluxDB export format
- [ ] Sampling strategies

#### D.2 Health Checks
```aether
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}

actor HealthChecker {
    var checks: Map<String, fn() -> HealthStatus> = Map::new();
    
    handle RegisterCheck(name: String, check: fn() -> HealthStatus) ! {alloc} {
        self.checks.insert(name, check);
    }
    
    handle CheckHealth() -> Map<String, HealthStatus> ! {io, alloc} {
        let mut results = Map::new();
        for (name, check_fn) in self.checks.iter() {
            let status = check_fn();
            results.insert(name, status);
        }
        return results;
    }
    
    handle GetStatus() -> OverallStatus ! {io} {
        let results = await this.ask(CheckHealth());
        let mut overall = OverallStatus::Healthy;
        
        for (_, status) in results.iter() {
            match status {
                HealthStatus::Unhealthy(_) => {
                    overall = OverallStatus::Unhealthy;
                    break;
                }
                HealthStatus::Degraded(_) if overall != OverallStatus::Unhealthy => {
                    overall = OverallStatus::Degraded;
                }
                _ => {}
            }
        }
        
        return overall;
    }
}
```

**Requirements**:
- [ ] Health check registration
- [ ] Async health checks
- [ ] Timeout enforcement
- [ ] Cascading health (parent depends on children)
- [ ] Liveness vs readiness distinction

---

### E. HOT CODE RELOADING

#### E.1 Module Hot Reload
```aether
actor ModuleHotReloader {
    var loaded_modules: Map<String, Module> = Map::new();
    
    handle ReloadModule(name: String, new_code: Vec<u8>) ! {io, alloc} {
        // 1. Compile new code
        let new_module = compile(new_code);
        
        // 2. Verify compatibility
        verify_module_compatibility(&self.loaded_modules[&name], &new_module);
        
        // 3. Migrate state from old to new
        let old_state = extract_state(&self.loaded_modules[&name]);
        apply_state(&new_module, old_state);
        
        // 4. Switch actors to new code
        for actor in get_actors_of_module(&name) {
            notify_code_update(actor, &new_module);
        }
        
        // 5. Update registry
        self.loaded_modules.insert(name, new_module);
    }
}

pub fn hot_reload_module(name: String, new_code: Vec<u8>) ! {io} {
    let reloader = get_module_reloader();
    reloader.send(ReloadModule { name: name, new_code: new_code });
}
```

**Requirements**:
- [ ] Code replacement without restart
- [ ] State migration between versions
- [ ] Atomic switchover
- [ ] Rollback capability
- [ ] Version tracking
- [ ] Compatibility checking

---

## III. PERFORMANCE TARGETS

| Aspect | Target | Verification |
|--------|--------|--------------|
| Message latency (local) | <1ms p99 | Latency benchmark |
| Message latency (remote) | <50ms p99 | Network benchmark |
| Actor creation | <10ms | Throughput test |
| Actor spawn rate | >100k/sec | Load test |
| Consensus latency | <100ms (Raft) | Consensus test |
| Memory per actor | <1KB baseline | Memory profile |
| Fault recovery | <1sec | Failover test |
| Trace overhead | <1% | Trace benchmark |

---

## IV. IMPLEMENTATION ROADMAP

**Week 1**: Raft consensus, Paxos variants  
**Week 2**: CRDT library, automatic convergence  
**Week 3**: Time-based scheduling, distributed tracing  
**Week 4**: Sharding and load balancing  
**Week 5**: Advanced supervision strategies  
**Week 6**: Dead-letter queues, metrics collection  
**Week 7**: Health checks, monitoring integration  
**Week 8**: Hot code reloading infrastructure  
**Week 9**: Integration testing across all features  
**Week 10**: Performance optimization, hardening  

---

## V. SUCCESS CRITERIA

✅ Aether handles distributed consensus correctly  
✅ Automatic fault recovery (any node can fail)  
✅ Zero message loss (at-least-once delivery)  
✅ Causality preservation (causal consistency)  
✅ Hot code reloading without data loss  
✅ Complete observability (tracing, metrics, health)  
✅ Performance within 10% of Akka  
✅ Can express Go, Erlang, Scala patterns  

**Aether becomes the distributed systems language to replace them all.**
