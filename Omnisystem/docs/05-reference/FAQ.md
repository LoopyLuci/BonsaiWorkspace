# Frequently Asked Questions

**Answers to common questions about Omnisystem**

---

## General Questions

**Q: What is Omnisystem?**  
A: An enterprise-grade computing platform with 4 integrated languages (TITAN, SYLVA, AETHER, AXIOM) covering systems programming, machine learning, distributed computing, and formal verification.

**Q: Can I use just one language?**  
A: Yes! Each language is independent. Start with TITAN for systems programming, SYLVA for ML, AETHER for distributed, or AXIOM for verification.

**Q: Is Omnisystem production-ready?**  
A: Yes. It's used by enterprises for high-reliability systems. Comprehensive testing, security audits, and formal verification available.

---

## Installation & Setup

**Q: How do I install Omnisystem?**  
A: Three methods:
1. Package manager: `choco install omnisystem` (Windows), `brew install omnisystem` (Mac)
2. Download binaries from omnisystem.io/download
3. Build from source: `git clone ... && cargo build --release`

**Q: What are system requirements?**  
A: 64-bit OS (Windows 10+, macOS 10.15+, Linux), 4GB RAM, 5GB disk space.

**Q: How do I set up my IDE?**  
A: Install official plugins for VS Code or JetBrains IDEs from their marketplaces.

---

## Language Selection

**Q: Which language should I use?**  
A: 
- Systems programming → TITAN
- Machine learning/AI → SYLVA  
- Distributed systems → AETHER
- Formal verification → AXIOM

**Q: Can I mix languages in one project?**  
A: Yes! Use bridges to call between languages. See [LANGUAGE_BRIDGES.md](LANGUAGE_BRIDGES.md).

**Q: How do I choose between TITAN and my current language?**  
A: TITAN is safer (memory safety, type system), more productive (less boilerplate), and faster to write. See [COMPARISON.md](COMPARISON.md).

---

## TITAN Specifics

**Q: Is TITAN similar to Rust?**  
A: Yes, conceptually similar with ownership and borrowing. TITAN is more accessible with better error messages and less boilerplate.

**Q: Can I write unsafe code?**  
A: Yes, but it's discouraged. TITAN's safe abstractions handle most cases. See [SECURITY.md](SECURITY.md).

**Q: Why would I use TITAN over Go?**  
A: Type safety, memory safety, better concurrency primitives, and performance.

---

## SYLVA Specifics

**Q: Can SYLVA do reinforcement learning?**  
A: Yes. SYLVA has built-in RL modules (Q-learning, policy gradient).

**Q: Does SYLVA support GPU?**  
A: Yes, CUDA and OpenCL support built-in. Enable with `export OMNISYSTEM_GPU=cuda`.

**Q: Can I train on distributed data?**  
A: Yes, use federated learning via AETHER bridge for distributed training.

---

## AETHER Specifics

**Q: What consensus does AETHER use?**  
A: Raft (default), Paxos, or Byzantine Fault Tolerant consensus. Choose based on your needs.

**Q: Can AETHER handle network partitions?**  
A: Yes, with automatic leader re-election. The smaller partition pauses to maintain consistency.

**Q: How many nodes minimum?**  
A: 3 for production (tolerates 1 failure). 5 for high availability (tolerates 2 failures).

---

## AXIOM Specifics

**Q: Can AXIOM prove my program correct?**  
A: Yes, with specifications. You provide preconditions, postconditions, and invariants. AXIOM proves or rejects.

**Q: Is automated proving slow?**  
A: For simple properties (10-100ms). Complex proofs may take seconds. Memoization helps.

**Q: Do I have to use AXIOM?**  
A: No, it's optional. Other languages have runtime checks. AXIOM adds mathematical certainty.

---

## Performance

**Q: How is TITAN's performance?**  
A: Comparable to Rust/C++. Most operations <1ms. Benchmarks at omnisystem.io/benchmarks.

**Q: Can SYLVA run on edge devices?**  
A: Yes, with quantization and pruning. Models can be 10-100x smaller.

**Q: What's AETHER's consensus latency?**  
A: 10-50ms for one round. Batching amortizes cost for 1000x throughput.

---

## Deployment

**Q: Can I deploy to cloud?**  
A: Yes. Docker, Kubernetes, and bare metal supported. See [DEPLOYMENT.md](DEPLOYMENT.md).

**Q: What about CI/CD?**  
A: GitHub Actions, GitLab CI, Jenkins, and others work with omnisystem CLI commands.

**Q: Can I hot-reload code?**  
A: Yes, with limitations. Stateless functions can be reloaded safely.

---

## Costs & Licensing

**Q: Is Omnisystem free?**  
A: Yes, completely free and open-source under permissive license.

**Q: Is there commercial support?**  
A: Yes, available from Omnisystems Inc. Visit omnisystem.io/support.

**Q: Can I use it commercially?**  
A: Yes, in any commercial application.

---

## Compatibility

**Q: Can I use existing C libraries?**  
A: Yes, via FFI (Foreign Function Interface). TITAN has seamless C interop.

**Q: Can I integrate with Python?**  
A: Yes, use bridges. SYLVA's Tensor can interop with NumPy/PyTorch.

**Q: What about Docker?**  
A: Official Docker image available. Multi-stage builds optimized.

---

## Troubleshooting

**Q: My code doesn't compile, how do I debug?**  
A: Run `omnisystem compile --verbose program.ti` for detailed errors.

**Q: Why is my program slow?**  
A: Use `omnisystem profile --cpu` to find bottlenecks. See [PERFORMANCE.md](PERFORMANCE.md).

**Q: How do I report bugs?**  
A: GitHub Issues at github.com/omnisystem/omnisystem. Include omnisystem diagnostics output.

---

## Learning Resources

**Q: How do I learn Omnisystem?**  
A: 
1. [INSTALLATION.md](INSTALLATION.md) - Setup
2. [HELLO_WORLD.md](HELLO_WORLD.md) - First programs
3. Language guides (TITAN, SYLVA, AETHER, AXIOM)
4. Tutorials (Web, ML, Distributed, Verification)

**Q: Are there examples?**  
A: Yes, in examples/ directory. Also check tutorials.

**Q: Is there a community?**  
A: Yes! Discord: discord.gg/omnisystem, Forum: omnisystem.io/forum

---

## Migration

**Q: Can I migrate from [other language]?**  
A: Yes. See [MIGRATION.md](MIGRATION.md) for language-specific guides.

**Q: Will my existing code work?**  
A: No, requires rewriting. But many patterns translate directly.

**Q: How long does migration take?**  
A: Varies. Simple programs: days. Complex systems: weeks to months.

---

## Still Have Questions?

- Check [GLOSSARY.md](GLOSSARY.md) for terminology
- Search community forum: omnisystem.io/forum
- Read relevant guide for your language
- Open GitHub issue with minimal reproducible example

---

**FAQ** - Get answers fast!
