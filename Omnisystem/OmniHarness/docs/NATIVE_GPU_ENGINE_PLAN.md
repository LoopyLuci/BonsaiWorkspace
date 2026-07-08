# Native GPU Inference Engine — Build Plan

## Status quo, verified for real this session

The `native`/`native-gpu` crate (`OmniHarness/native/src/lib.rs`) is a
one-line placeholder — no Vulkan/compute code exists there. Today's real
inference path is: `model_orchestrator.rs` spawns `llama-server.exe`
(bootstrapped from the upstream llama.cpp GitHub release,
`bootstrap.rs`) as a subprocess, and talks to it over HTTP
(`reqwest` → `http://127.0.0.1:{port}/completion`). That subprocess links
`ggml-vulkan.dll` and does all real GPU work internally.

**Verified live this session, on this machine's actual AMD RX 7900 XTX:**
`llama-server.exe --list-devices` → `Vulkan0: AMD Radeon RX 7900 XTX (24560
MiB, 23735 MiB free)`. A real completion request against
`Bonsai-1.7B-Q2_K.gguf` with `--n-gpu-layers 99` produced **186–217
tokens/sec**, with Windows GPU performance counters confirming **64% GPU
compute-engine utilization attributed specifically to the llama-server
process** (`Get-Counter '\GPU Engine(*)\Utilization Percentage'` filtered by
PID). GPU acceleration genuinely works today — this is not a paper
capability.

## The actual question this plan answers

"Build a native Vulkan engine, as fast and optimal as possible" has two
readings, and they lead to very different plans:

1. **Write our own Vulkan compute kernels from scratch** (matmul, attention,
   quantized dequant, KV-cache management, per architecture) to replace
   llama.cpp entirely.
2. **Bind directly to the engine that's already proven fast on this exact
   hardware** (ggml/llama.cpp's C API via FFI), eliminating the
   subprocess+HTTP layer around it, but keeping its GPU kernels.

Option 1 means re-deriving, from zero, kernel-level work that llama.cpp's
`ggml-vulkan` backend already has years of contributor-hours and
hardware-specific tuning behind (including AMD/RDNA3-specific code paths) —
already measured at 186+ tok/s on this GPU. Matching that from scratch is a
multi-month-to-multi-year undertaking for a large team, and the realistic
near-term result would be **slower** than what already runs today. That is
the opposite of "fast and optimal."

**Recommendation: Option 2.** "Native" doesn't have to mean "kernels we
wrote" — it means "runs in-process, not as a subprocess we talk to over
HTTP." FFI-binding to llama.cpp/ggml gets a genuinely native engine (no
process spawn, no JSON/HTTP round-trip, direct memory access, direct
streaming callbacks) while inheriting the exact GPU performance already
verified above, for a fraction of the effort. This is the standard approach
other real Rust LLM projects (`llama-cpp-2`, `llama-cpp-rs`) already take,
for the same reason.

## What "native" actually buys, concretely

The subprocess+HTTP approach's real costs, worth quantifying before
building anything (Phase 0 below measures these on this exact machine
rather than assuming them):
- Process spawn + model load time per slot (currently paid on every model
  switch).
- HTTP request/response JSON serialization overhead per token/per request.
- No shared GPU/model memory between logical "slots" beyond what
  llama-server's own multi-slot support already provides in one process.
- Harder to do tight integration (e.g. per-token callback into Rust code
  without a streaming-HTTP parser in between).

An in-process FFI engine removes all of these, without touching the GPU
kernel layer at all.

## Phased plan

| Phase | Goal | Real work | Depends on |
|---|---|---|---|
| 0 | Baseline measurement | Benchmark current subprocess+HTTP path: cold model-load time, per-request latency floor, tokens/sec (already have one data point: 186–217 t/s on Q2_K). Establishes the actual number Phase 4 needs to beat. | none |
| 1 | FFI bindings | `bindgen`-generate Rust bindings to `llama.h`/`ggml.h`'s public C API (context creation, model load, tokenize, decode, sampling) against the same llama.cpp source version already bootstrapped (`bootstrap.rs` pins a release tag — build the bindings against that exact tag, not `HEAD`, so the vendored binary and the FFI surface never drift apart). Link against the *already-verified-working* `llama.dll`/`ggml-vulkan.dll` in the sidecars directory — no new compute code, just a typed Rust surface over it. | Phase 0 (know what to beat) |
| 2 | Minimal in-process engine | New crate (e.g. `OmniHarness/native`, replacing the placeholder) wrapping the FFI: load a model, run one completion, stream tokens via a Rust callback/channel instead of parsing SSE/HTTP chunks. Reuse `gguf_tokenizer.rs` (built this session, verified against the real Bonsai file) to cross-check tokenization matches llama.cpp's own internal tokenizer exactly — a real, cheap correctness gate before trusting the FFI path for anything user-facing. | Phase 1 |
| 3 | Feature parity | GPU-layer control (`--n-gpu-layers` equivalent), context size, sampling parameters, multi-sequence/slot support — matching what `model_orchestrator.rs` already exposes today, so this can be a drop-in alternative backend, not a parallel feature set to maintain. | Phase 2 |
| 4 | Head-to-head benchmark | Same prompt/model/GPU-layer config, in-process vs. subprocess, measured on this machine. Only promote the native path to default once it's measurably better (Phase 0's baseline exists specifically so this isn't a vibes-based decision). | Phase 3 |
| 5 | Cutover | `model_orchestrator.rs` gains a native-engine code path behind the existing `native-gpu` feature flag (already wired, currently a no-op) — subprocess approach stays as the default/fallback until Phase 4's numbers justify flipping the default, and stays available permanently as a safety net (a crash in an in-process FFI engine takes down the whole app; a crashed subprocess doesn't). | Phase 4 |

## Explicitly not in scope

- Writing new Vulkan compute kernels. Nothing above requires it — the whole
  point is reusing the kernels already measured working.
- Dropping the subprocess path. It's the safer default (crash isolation) and
  should remain available even after Phase 5.
- Supporting non-AMD/non-Vulkan backends as part of this plan — CUDA/Metal
  are already handled by llama.cpp itself if the bootstrapped binary
  supports them; this plan doesn't change that.

## Verification per phase

- Phase 0: numbers written down, not estimated — this is the plan's own
  "don't guess, measure" principle applied to itself.
- Phase 1: a trivial FFI smoke test (load a small model, run
  `llama_tokenize`, assert non-empty output) — proves the binding links and
  calls correctly before any inference logic is built on top.
- Phase 2: `gguf_tokenizer.rs`-vs-FFI-tokenizer cross-check on real text
  (exact match required, not "close enough") before trusting generated
  token IDs for anything else.
- Phase 4: the actual go/no-go gate — numbers must show a real improvement
  over Phase 0's baseline, on the same hardware, same model, same
  GPU-layer count.
