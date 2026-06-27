# Glossary of Terms

**Common Omnisystem terminology and concepts**

---

## A

**AETHER**: Distributed systems language for consensus-based computing and replication

**Arc**: Atomic Reference Counted pointer for shared ownership across threads

**Axiom**: Fundamental assumption or proven statement used as basis for proofs

**AXIOM**: Formal verification language for theorem proving and correctness proofs

---

## B

**Base Module**: Required core module (11 total) that all applications depend on

**BFT**: Byzantine Fault Tolerant consensus, tolerates up to 1/3 malicious nodes

**Borrow Checker**: TITAN mechanism that ensures memory safety through ownership rules

**Box**: Heap-allocated pointer for owned values

---

## C

**Cluster**: Group of distributed nodes running consensus protocol

**Consensus**: Agreement mechanism ensuring all nodes have consistent state

**Conv2d**: 2D Convolutional neural network layer for image processing

**CRUD**: Create, Read, Update, Delete operations on data

---

## D

**Dense**: Fully-connected neural network layer

**Distributed Ledger**: Replicated data structure across multiple nodes

**DSL**: Domain-Specific Language for specialized use cases

---

## E

**Enum**: Type with multiple variants (enumeration)

**Epoch**: One complete pass through training data in ML

**Erasure Code**: Technique to recover lost data from multiple copies

---

## F

**FFI**: Foreign Function Interface for calling code from other languages

**Futures**: Asynchronous computation that will be resolved later

---

## G

**Generic**: Type parameter that works with multiple concrete types

**GC**: Garbage Collection for automatic memory management

**Gradient**: Rate of change used in backpropagation for ML training

---

## H

**Hashmap**: Data structure for key-value storage with O(1) lookup

**Heap**: Region of memory for dynamic allocation

**Hyperparameter**: Configuration parameter of ML model (learning rate, batch size)

---

## I

**Inference**: Running a trained ML model on new data for predictions

**LSTM**: Long Short-Term Memory recurrent neural network layer

---

## J

**JIT**: Just-In-Time compilation that compiles code during execution

**JSON**: JavaScript Object Notation, human-readable data format

---

## K

**Key-Value Store**: Database storing pairs of keys and associated values

---

## L

**Leader**: Node elected to coordinate consensus in distributed system

**Loss**: Measure of model prediction error, minimized during training

**LSP**: Language Server Protocol for IDE integration

---

## M

**Macro**: Code template that generates code at compile-time

**Middleware**: Code that processes requests/responses between client and server

**Mutex**: Mutual Exclusion lock for thread-safe access

---

## N

**Node**: Single machine/process in a distributed system

**NaN**: Not a Number, result of invalid floating-point operation

---

## O

**OMNI**: Universal format for cross-language data serialization

**Ownership**: TITAN concept where each value has one owner responsible for cleanup

**Optimizer**: Algorithm that adjusts model weights to minimize loss (Adam, SGD)

---

## P

**Paxos**: Consensus algorithm using multi-round voting

**Panic**: Unrecoverable error that terminates program

**Partition**: Split data across multiple nodes in distributed system

**Proof**: Logical derivation showing theorem is true

---

## Q

**Quorum**: Minimum majority required for consensus (n/2 + 1)

---

## R

**Raft**: Consensus algorithm easier to understand than Paxos

**RPC**: Remote Procedure Call to invoke function on remote node

**RAII**: Resource Acquisition Is Initialization, acquire in constructor, release in destructor

---

## S

**Schema**: Structure defining format of data

**Shard**: Partition of data distributed across cluster

**Softmax**: Activation function converting logits to probabilities

**Span**: Distributed trace representing operation across services

**Stack**: Region of memory for function calls and local variables

**Struct**: Composite type grouping multiple fields

**SYLVA**: Machine learning language for neural networks and AI

---

## T

**Tensor**: N-dimensional array fundamental to ML/SYLVA

**TITAN**: Systems programming language with memory safety

**Trait**: Contract defining methods types must implement

**Transaction**: Atomic operation that either fully completes or fully rolls back

---

## U

**Universal Module**: Optional enhancement module (52 total) extending base functionality

---

## V

**Vec**: Dynamic array that grows/shrinks at runtime

**Variance**: Measure of spread in data

---

## W

**Webhook**: HTTP callback triggered by specific events

**Workload**: Pattern of requests and operations typical of application

---

## X

**XOR**: Exclusive OR logical operation

---

## Z

**ZK-Proof**: Zero-Knowledge Proof proving statement without revealing data

---

## Acronyms

| Acronym | Full Form |
|---------|-----------|
| API | Application Programming Interface |
| CLI | Command-Line Interface |
| CPU | Central Processing Unit |
| CRUD | Create, Read, Update, Delete |
| FFI | Foreign Function Interface |
| GC | Garbage Collection |
| GPU | Graphics Processing Unit |
| HTTP | HyperText Transfer Protocol |
| IDL | Interface Definition Language |
| JIT | Just-In-Time |
| JSON | JavaScript Object Notation |
| LSP | Language Server Protocol |
| ML | Machine Learning |
| ORM | Object-Relational Mapping |
| RAII | Resource Acquisition Is Initialization |
| RPC | Remote Procedure Call |
| SQL | Structured Query Language |
| URI | Uniform Resource Identifier |
| UUID | Universally Unique Identifier |
| XML | Extensible Markup Language |

---

## Symbols & Operators

| Symbol | Name | Usage |
|--------|------|-------|
| `&` | Reference/Borrow | `&x` - immutable borrow |
| `&mut` | Mutable Borrow | `&mut x` - mutable borrow |
| `*` | Dereference | `*ptr` - access pointed value |
| `?` | Try Operator | `value?` - propagate error |
| `::` | Path Separator | `std::collections::HashMap` |
| `->` | Return Type | `fn foo() -> i32` |
| `=>` | Match Arm | `Some(x) => x + 1` |
| `..` | Range | `0..10` - range 0 to 9 |

---

**Glossary** - Your reference for Omnisystem terminology!
