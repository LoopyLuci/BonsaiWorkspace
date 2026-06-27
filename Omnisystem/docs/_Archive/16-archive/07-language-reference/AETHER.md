# Aether Language Reference

**Aether** is the distributed application language of the Omnisystem. It sits one level above Titan in the language stack. Where Titan gives you bit-level control, Aether gives you actor-level concurrency: isolated processes that communicate exclusively through typed messages, with supervision trees that automatically recover from failure, and built-in replicated data types that converge without coordination.

Aether code compiles through UniIR to native machine code via the same pipeline as Titan. The actor runtime is part of OmniCore, not an external library.

---

## 1. Design Principles

1. **No shared mutable state.** Actors communicate only through messages. An actor's internal state is never directly accessible from outside. Data races are architecturally impossible.
2. **Failure is normal.** Every actor runs under a supervisor. When an actor crashes, the supervisor restarts it according to its strategy. No actor failure propagates silently.
3. **Location transparency.** The code to send a message to a local actor is identical to sending a message to an actor on a remote node. The runtime handles routing.
4. **Effects declared.** Aether inherits Titan's effect system. Network calls, allocations, and telemetry all appear in function signatures.
5. **Convergence built in.** Distributed state uses CRDTs (Conflict-free Replicated Data Types) that merge automatically without coordination or locks.

---

## 2. Actors

An actor is a self-contained unit of computation with private state and a message queue (mailbox). Messages are processed one at a time, so actors are inherently thread-safe without locks.

### 2.1 Defining an Actor

```aether
actor CounterActor {
    var count: i64 = 0;

    on_start() {
        emit telemetry { event: "counter_started" };
    }

    handle Increment(by: i64) {
        self.count = self.count + by;
    }

    handle GetValue() -> i64 {
        return self.count;
    }

    handle Reset() {
        self.count = 0;
    }

    on_stop() {
        emit telemetry { event: "counter_stopped", final_value: self.count };
    }
}
```

### 2.2 Actor Anatomy

| Section | Keyword | Purpose |
|---------|---------|---------|
| State | `var` | Private mutable state, initialized once |
| Lifecycle start | `on_start()` | Runs when actor is first spawned |
| Message handler | `handle MsgName(params) [-> RetType]` | Processes one message type |
| Lifecycle stop | `on_stop()` | Runs when actor is stopped or crashed |
| Lifecycle restart | `on_restart(reason)` | Runs after crash before resuming |

### 2.3 Spawning Actors

```aether
// Spawn on local node
let counter = spawn(CounterActor);

// Spawn on specific node
let remote_counter = spawn_on("node-2", CounterActor);

// Spawn with initial arguments
let server = spawn(HttpServer { port: 8080 });
```

### 2.4 Sending Messages

```aether
// Fire-and-forget (asynchronous)
counter.send(Increment { by: 1 });

// Request-reply (asynchronous with await)
let val = await counter.ask(GetValue {});

// Broadcast to all actors of a type
broadcast(CounterActor, Reset {});
```

### 2.5 Self-Reference

Inside a handler, `self` refers to the actor's own state. `this` refers to the actor's own reference (for sending messages to itself):

```aether
handle Schedule(delay_ms: i64) {
    // send a message to ourselves after a delay
    after(delay_ms, this, Tick {});
}
```

### 2.6 Stopping an Actor

```aether
handle Shutdown() {
    stop();    // graceful stop — runs on_stop() before terminating
}
```

---

## 3. Messages

Messages are typed structs. Every message type must be declared before use:

```aether
message Deposit { amount: i64 }
message Withdraw { amount: i64 }
message GetBalance()
message BalanceResponse { value: i64 }
```

Messages are immutable once sent. The runtime serializes them for remote delivery.

### 3.1 Pattern Matching on Messages

When an actor may receive multiple message types, use a combined handler with `match`:

```aether
actor BankAccount {
    var balance: i64 = 0;

    handle Any(msg) {
        match msg {
            Deposit { amount }   => { self.balance = self.balance + amount; }
            Withdraw { amount }  => {
                if amount <= self.balance {
                    self.balance = self.balance - amount;
                }
            }
            GetBalance {}        => { send BalanceResponse { value: self.balance }; }
        }
    }
}
```

---

## 4. Supervision

Every actor in Aether runs under a supervisor. The supervisor monitors its children and restarts them when they crash.

### 4.1 Supervision Strategies

| Strategy | Keyword | Behavior on child crash |
|----------|---------|------------------------|
| One for one | `ONE_FOR_ONE` | Restart only the crashed child |
| One for all | `ONE_FOR_ALL` | Restart all children |
| Rest for one | `REST_FOR_ONE` | Restart the crashed child and all children spawned after it |

### 4.2 Supervision Tree

```aether
fn start_application() ! { alloc, network("*"), io } {
    let root = Supervisor::new(
        strategy: ONE_FOR_ONE,
        max_restarts: 5,
        restart_window_ms: 60000,
        children: [
            ChildSpec {
                name: "database",
                actor: DatabaseActor,
                restart: PERMANENT,       // always restart
                restart_cooldown_ms: 500,
            },
            ChildSpec {
                name: "http_server",
                actor: HttpServerActor { port: 8080 },
                restart: TRANSIENT,       // restart only on abnormal exit
                restart_cooldown_ms: 1000,
            },
            ChildSpec {
                name: "cache",
                actor: CacheActor,
                restart: TEMPORARY,       // never restart
                restart_cooldown_ms: 0,
            },
        ],
    );
    root.start();
}
```

### 4.3 Restart Policies

| Policy | Keyword | When restarted |
|--------|---------|----------------|
| Always | `PERMANENT` | On any termination, normal or abnormal |
| On failure | `TRANSIENT` | Only on crash (abnormal exit) |
| Never | `TEMPORARY` | Never restarted |

### 4.4 Actor Failure Handling

When an actor panics or exceeds resource limits, its supervisor receives a `ChildFailed` signal. If the restart count within the window exceeds `max_restarts`, the supervisor itself fails, propagating up the tree:

```aether
actor RobustWorker {
    on_restart(reason: str) {
        // Clean up state before resuming after a crash
        self.reset_internal_state();
        emit telemetry { event: "restarted", reason: reason };
    }

    handle DangerousWork(data: str) {
        // If this panics, supervisor will restart RobustWorker
        process(data);
    }
}
```

---

## 5. Consistency Models

Distributed state in Aether is typed by its consistency guarantee. The type system prevents accidentally mixing consistency levels.

### 5.1 Consistent (Linearizable)

Single-writer or consensus-backed. All reads see all prior writes in global order:

```aether
var config: Consistent<Config> = Consistent::new(Config::default());

// Reading always returns the latest committed value
let current = config.read();

// Writing requires exclusive access or consensus
config.write(new_config);
```

Use `Consistent` for configuration, primary keys, and anything requiring correctness over availability.

### 5.2 Eventually Consistent (CRDT)

Conflict-free. Writes never block; state converges automatically across replicas:

```aether
var visit_count: Eventually<GCounter> = GCounter::new(node_id);

handle RecordVisit() {
    self.visit_count.increment(1);    // never blocks, always succeeds
}

handle GetApproxCount() -> i64 {
    return self.visit_count.value();  // may be slightly stale
}
```

Use `Eventually` for counters, sets, and append-only logs where temporary inconsistency is acceptable.

### 5.3 Causal Consistency

Preserves happens-before relationships. If you saw write A before write B, everyone who sees B also sees A:

```aether
var timeline: Causal<Vec<Event>> = Causal::new();

handle PostEvent(e: Event) {
    self.timeline.append(e);
}
```

### 5.4 Bounded Staleness

Guarantees a maximum lag window between replicas:

```aether
var cache: BoundedStaleness<Config> = BoundedStaleness::new(
    max_lag_ms: 5000
);
```

---

## 6. CRDT Primitives

Aether provides built-in replicated data types that merge without coordination:

### 6.1 GCounter (Grow-only Counter)

```aether
var counter: GCounter = GCounter::zero();

// Increment (always succeeds, never rolls back)
counter.increment(1);
counter.increment(by: 5);

// Read current value
let total = counter.value();

// Merge with replica from another node
counter.merge(remote_counter);

// Delta sync (only changed values)
let delta = counter.delta();
counter.apply_delta(peer_delta);
```

Properties: commutative, idempotent, associative. Safe to merge in any order.

### 6.2 GSet (Grow-only Set)

```aether
var seen_ids: GSet<str> = GSet::new();
seen_ids.add("user-123");
let contains = seen_ids.contains("user-123");  // true
// Elements can never be removed from a GSet
```

### 6.3 ORMap (Observed-Remove Map)

Allows adding and removing entries. Concurrent adds win over removes:

```aether
var sessions: ORMap<str, Session> = ORMap::new();
sessions.put("session-1", session_data);
sessions.remove("session-1");
let s = sessions.get("session-1");    // None after remove
```

### 6.4 LWWRegister (Last-Writer-Wins Register)

Single value with timestamp-based conflict resolution:

```aether
var nickname: LWWRegister<str> = LWWRegister::new();
nickname.set("Alice", timestamp: now());
let current = nickname.get();
```

---

## 7. Node API (Runtime)

The `ActorNode` is the runtime entry point. One node per process:

```python
# Python embedding (for integration and testing)
from aether.runtime.node import ActorNode

node = ActorNode(
    node_id="my-node",
    bind_host="127.0.0.1",
    bind_port=9000,
)
node.start()

# Spawn an actor
ref = node.spawn(MyActor)

# Send a message
ref.send({"type": "Increment", "by": 1})

# Get supervision tree state
tree = node.get_supervision_tree()

# Graceful shutdown
node.shutdown()
```

### 7.1 Multi-Node Cluster

```aether
fn join_cluster() ! { network("*"), alloc } {
    let node = ActorNode::new(
        node_id: "worker-1",
        bind_host: "0.0.0.0",
        bind_port: 9001,
    );

    // Connect to seed peers
    node.connect("192.168.1.10:9000");
    node.connect("192.168.1.11:9000");

    node.start();
}
```

---

## 8. Telemetry

Every actor lifecycle event and message handler can emit structured telemetry:

```aether
actor TrackedWorker {
    handle Process(task: Task) {
        let start = now();
        let result = do_work(task);
        emit telemetry {
            event: "task_completed",
            task_id: task.id,
            duration_ms: now() - start,
            success: result.is_ok(),
        };
    }
}
```

Telemetry events are content-addressed (Blake3 hash) and queryable through `build observe`.

---

## 9. Network Transport

Aether uses a TCP-based transport for actor messages. The protocol is:

- **Wire format:** CBOR-encoded `ActorMessage` structs
- **Content addressing:** Each message has a SHA-256 hash for integrity verification
- **Actor registry:** Gossip protocol for location discovery — actors are found by ID, not by address
- **Reconnection:** Automatic with exponential backoff

The DHT package registry (separate from the actor registry) uses its own TCP JSON protocol. Do not attempt to unify them — the actor transport is fire-and-forget; the DHT is iterative request/response.

---

## 10. Complete Example: Distributed Counter Service

```aether
// Message types
message Increment { by: i64 }
message Decrement { by: i64 }
message GetValue()
message ValueResponse { value: i64 }

// Main counter actor
actor DistributedCounter {
    var local_count: GCounter = GCounter::zero();
    var node_id: str = "";

    on_start() {
        self.node_id = self_node_id();
        emit telemetry { event: "counter_ready", node: self.node_id };
    }

    handle Increment(by: i64) {
        self.local_count.increment(by);
    }

    handle Decrement(by: i64) {
        // GCounter can only grow — model decrement as negative increment
        // on a PN-Counter (positive-negative pair)
        // (For simplicity here, clamp to 0)
    }

    handle GetValue() {
        let v = self.local_count.value();
        send ValueResponse { value: v };
    }

    handle SyncDelta(delta: GCounterDelta) {
        self.local_count.apply_delta(delta);
    }
}

// Supervisor
fn start_counter_service() ! { alloc, network("*") } {
    let supervisor = Supervisor::new(
        strategy: ONE_FOR_ONE,
        max_restarts: 10,
        restart_window_ms: 30000,
        children: [
            ChildSpec {
                name: "counter",
                actor: DistributedCounter,
                restart: PERMANENT,
                restart_cooldown_ms: 100,
            },
        ],
    );
    supervisor.start();
}
```

---

## 11. Quick Reference Card

```
ACTOR KEYWORDS    actor  handle  on_start  on_stop  on_restart  var
MESSAGING         send  await  ask  broadcast  spawn  spawn_on  stop
SUPERVISION       ONE_FOR_ONE  ONE_FOR_ALL  REST_FOR_ONE
RESTART POLICIES  PERMANENT  TRANSIENT  TEMPORARY
CONSISTENCY       Consistent  Eventually  Causal  BoundedStaleness
CRDTS             GCounter  GSet  ORMap  LWWRegister
CRDT OPERATIONS   .increment()  .value()  .merge()  .delta()  .apply_delta()
                  .add()  .contains()  .put()  .remove()  .get()  .set()
TELEMETRY         emit telemetry { event: "...", key: value, ... }
EFFECTS           alloc  io  network("pattern")  telemetry  panic
```
