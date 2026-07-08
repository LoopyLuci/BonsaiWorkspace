"""OmniHarness FastAPI HTTP server — REST API and WebSocket streaming."""
from __future__ import annotations

import asyncio
import json
import logging
import time
import uuid
from contextlib import asynccontextmanager
from typing import AsyncIterator

logger = logging.getLogger("omniharness.server")

from fastapi import FastAPI, HTTPException, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import StreamingResponse
from pydantic import BaseModel, Field
from sse_starlette.sse import EventSourceResponse

from .models.router import ModelRouter
from .models.base import ChatMessage, ChatRequest, ToolDef
from .substrate import (
    Governor, Budget, CapabilityPolicy, SwarmCoordinator, AgentSpec,
    RagPipeline, run_ensemble, DistillationDatasetBuilder,
)
from .react.engine import ReActEngine
from .react.tools import get_registry
from .memory.vector import VectorClient
from .memory.episodic import EpisodicMemory
from .memory.graph import KnowledgeGraph
from .grpc_client import GrpcClient
from .clj_client import CljClient

# ── Globals ───────────────────────────────────────────────────────────────────

router:    ModelRouter   | None = None
react_eng: ReActEngine   | None = None
vec_store: VectorClient  | None = None
episodic:  EpisodicMemory | None = None
graph:     KnowledgeGraph | None = None
grpc:      GrpcClient    | None = None
clj:       CljClient     = CljClient()  # HTTP, connectionless — no connect/close needed

@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncIterator[None]:
    global router, react_eng, vec_store, episodic, graph, grpc
    import dotenv; dotenv.load_dotenv()

    router    = ModelRouter()
    router.register_from_env()
    # Zero-config: pick up any locally-running Ollama / LM Studio / llama.cpp
    # runtime automatically, so a user with a local model installed can chat
    # immediately without setting any env var or key.
    router.autodiscover_local(force=True)

    tools     = get_registry()
    react_eng = ReActEngine(router, tools)

    vec_store = VectorClient()
    episodic  = EpisodicMemory()
    await episodic.init()
    graph     = KnowledgeGraph()

    grpc = GrpcClient()
    try:
        await grpc.connect()
    except Exception:
        logger.exception("Kernel gRPC client failed to initialize — degrading to kernel-less mode")
        grpc = None  # kernel not running — graceful degradation

    yield

    await episodic.close()
    if grpc:
        await grpc.close()

# ── App ───────────────────────────────────────────────────────────────────────

app = FastAPI(
    title="OmniHarness Orchestrator",
    version="1.0.0",
    description="Polyglot AI harness — any model, any tool, any memory",
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# ── Pydantic models ───────────────────────────────────────────────────────────

class ChatReq(BaseModel):
    model_id:    str                    = "claude-sonnet-4-6"
    messages:    list[dict]             = []
    temperature: float                  = 0.7
    max_tokens:  int                    = 4096
    system:      str | None             = None
    session_id:  str | None             = None
    stream:      bool                   = False
    # Native function calling: list of {name, description, parameters(JSON Schema)}.
    tools:       list[dict] | None      = None

class AgentReq(BaseModel):
    objective:   str
    model_id:    str   = "claude-sonnet-4-6"
    max_steps:   int   = 20
    temperature: float = 0.7

class MemoryStoreReq(BaseModel):
    collection: str   = "default"
    content:    str
    metadata:   dict  = {}

class MemorySearchReq(BaseModel):
    collection: str   = "default"
    query:      str
    top_k:      int   = 5
    threshold:  float = 0.0

class SessionCreateReq(BaseModel):
    title:    str = "New Session"
    model_id: str = "claude-sonnet-4-6"

class ToolExecReq(BaseModel):
    name:      str
    arguments: dict = {}

# ── Health ────────────────────────────────────────────────────────────────────

@app.get("/api/health")
async def health():
    providers = {}
    if router:
        for name in router.list_providers():
            try:
                ok = await router.health(name)
                providers[name] = ok
            except Exception:
                providers[name] = False
    kernel_ok = False
    if grpc:
        try:
            status = await grpc.status()
            kernel_ok = status.get("healthy", False)
        except Exception:
            logger.exception("Kernel gRPC health check failed")
    clj_ok = await clj.health()
    return {"status": "ok", "providers": providers, "kernel": kernel_ok, "clj_orchestrator": clj_ok, "version": "1.0.0"}

# ── Clojure orchestrator (HTN planner / policy engine) ──────────────────────────

class PlanReq(BaseModel):
    task_name: str
    params: dict = {}

class PolicyCheckReq(BaseModel):
    action: str
    args: dict = {}

@app.post("/api/planner/plan")
async def planner_plan(req: PlanReq):
    """HTN plan via clj-orchestrator. 503 if it isn't running — this is an
    optional sidecar, not a required dependency of the orchestrator."""
    result = await clj.plan(req.task_name, req.params)
    if result is None:
        raise HTTPException(503, "clj-orchestrator not reachable (start it: cd clj-orchestrator && lein run serve)")
    return result

@app.post("/api/planner/execute")
async def planner_execute(req: PlanReq):
    result = await clj.plan_execute(req.task_name, req.params)
    if result is None:
        raise HTTPException(503, "clj-orchestrator not reachable (start it: cd clj-orchestrator && lein run serve)")
    return result

@app.post("/api/planner/policy-check")
async def planner_policy_check(req: PolicyCheckReq):
    result = await clj.policy_check(req.action, req.args)
    if result is None:
        raise HTTPException(503, "clj-orchestrator not reachable (start it: cd clj-orchestrator && lein run serve)")
    return result

# ── Models ────────────────────────────────────────────────────────────────────

@app.get("/api/models")
async def list_models(provider: str = ""):
    if not router:
        return {"models": []}
    # Re-probe for local runtimes (throttled) so models from an Ollama/LM Studio
    # instance started after the server came up appear on the next refresh.
    router.autodiscover_local()
    models = router.list_all_models()
    if provider:
        models = [m for m in models if m.provider == provider]
    return {"models": [m.__dict__ for m in models]}

# ── Chat ──────────────────────────────────────────────────────────────────────

@app.post("/api/chat")
async def chat(req: ChatReq):
    if not router:
        raise HTTPException(503, "Router not initialized")

    # Preserve full message fields (tool_calls, tool_call_id, name) so native
    # function-calling round-trips work across turns.
    msgs = [ChatMessage(**m) for m in req.messages]
    if req.session_id and episodic:
        history = await episodic.get_history(req.session_id)
        msgs    = history + msgs

    tools = [ToolDef(**t) for t in req.tools] if req.tools else None

    chat_req = ChatRequest(
        model_id=req.model_id, messages=msgs,
        temperature=req.temperature, max_tokens=req.max_tokens,
        system=req.system, tools=tools,
    )

    resp = await router.chat(chat_req)

    if req.session_id and episodic:
        if req.messages:
            last = req.messages[-1]
            await episodic.add_turn(req.session_id, last["role"], last["content"])
        await episodic.add_turn(req.session_id, "assistant", resp.content)

    if grpc:
        try:
            await grpc.append_event("orchestrator", "ChatComplete",
                {"model": resp.model_used, "tokens_out": resp.output_tokens},
                req.session_id or "")
        except Exception:
            pass

    return {
        "content":       resp.content,
        "model_used":    resp.model_used,
        "finish_reason": resp.finish_reason,
        "input_tokens":  resp.input_tokens,
        "output_tokens": resp.output_tokens,
        "latency_ms":    resp.latency_ms,
        "tool_calls":    [tc.model_dump() for tc in resp.tool_calls],
    }

@app.post("/api/chat/stream")
async def chat_stream(req: ChatReq):
    if not router:
        raise HTTPException(503, "Router not initialized")
    msgs     = [ChatMessage(role=m["role"], content=m["content"]) for m in req.messages]
    chat_req = ChatRequest(
        model_id=req.model_id, messages=msgs,
        temperature=req.temperature, max_tokens=req.max_tokens,
        system=req.system, stream=True,
    )

    async def generate():
        async for chunk in router.stream(chat_req):
            yield f"data: {json.dumps({'delta': chunk})}\n\n"
        yield "data: [DONE]\n\n"

    return StreamingResponse(generate(), media_type="text/event-stream")

# ── Agent / ReAct ─────────────────────────────────────────────────────────────

@app.post("/api/agent/run")
async def agent_run(req: AgentReq):
    if not react_eng:
        raise HTTPException(503, "ReAct engine not initialized")
    result = await react_eng.run(
        objective=req.objective,
        model_id=req.model_id,
        max_steps=req.max_steps,
        temperature=req.temperature,
    )
    return {
        "answer":       result.answer,
        "success":      result.success,
        "steps":        [vars(s) for s in result.steps],
        "total_tokens": result.total_tokens,
        "elapsed_ms":   result.elapsed_ms,
    }

# ── Sessions ──────────────────────────────────────────────────────────────────

@app.post("/api/sessions")
async def create_session(req: SessionCreateReq):
    if episodic is None:
        raise HTTPException(503, "Episodic memory not initialized")
    import uuid
    sid = str(uuid.uuid4())
    return {"id": sid, "title": req.title, "model_id": req.model_id}

@app.get("/api/sessions/{session_id}")
async def get_session(session_id: str):
    if not episodic:
        raise HTTPException(503, "Episodic memory not initialized")
    history = await episodic.get_history(session_id)
    return {"id": session_id, "history": [{"role": m.role, "content": m.content} for m in history]}

@app.get("/api/sessions")
async def list_sessions():
    if not episodic:
        return {"sessions": []}
    sessions = await episodic.get_all_sessions()
    return {"sessions": sessions}

@app.delete("/api/sessions/{session_id}")
async def delete_session(session_id: str):
    if not episodic:
        raise HTTPException(503, "Episodic memory not initialized")
    count = await episodic.delete_session(session_id)
    return {"deleted_turns": count}

@app.post("/api/sessions/{session_id}/messages")
async def session_message(session_id: str, req: ChatReq):
    req.session_id = session_id
    return await chat(req)

# ── Memory ────────────────────────────────────────────────────────────────────

@app.post("/api/memory/store")
async def memory_store(req: MemoryStoreReq):
    if not vec_store:
        raise HTTPException(503, "Vector store not initialized")
    eid = await vec_store.store(req.collection, req.content, req.metadata)
    return {"id": eid, "collection": req.collection}

@app.post("/api/memory/search")
async def memory_search(req: MemorySearchReq):
    if not vec_store:
        raise HTTPException(503, "Vector store not initialized")
    results = await vec_store.search(req.collection, req.query, req.top_k, req.threshold)
    return {"results": [{"id": e.id, "content": e.content, "score": e.score, "metadata": e.metadata} for e in results]}

@app.get("/api/memory/collections")
async def memory_collections():
    if not vec_store:
        return {"collections": []}
    return {"collections": await vec_store.list_collections()}

# ── Tools ─────────────────────────────────────────────────────────────────────

@app.get("/api/tools")
async def list_tools():
    reg = get_registry()
    return {"tools": [{"name": t.name, "description": t.description} for t in reg.list_all()]}

@app.post("/api/tools/execute")
async def execute_tool(req: ToolExecReq):
    reg    = get_registry()
    result = await reg.execute(req.name, req.arguments)
    return {"result": result, "tool": req.name}

# ── Knowledge Graph ───────────────────────────────────────────────────────────

@app.post("/api/graph/extract")
async def graph_extract(body: dict):
    text    = body.get("text", "")
    nodes   = graph.extract_entities(text)
    return {"nodes": [{"id": n.id, "label": n.label, "type": n.type} for n in nodes]}

@app.get("/api/graph")
async def graph_stats():
    return graph.stats()

@app.get("/api/graph/export")
async def graph_export():
    return json.loads(graph.export_json())

# ── Substrate: swarm / ensemble / RAG / distillation ──────────────────────────
#
# The substrate engines are provider-agnostic (they take an injected LLM function).
# Here we wire that function to the live ModelRouter, and wrap every run in a
# Governor that enforces budgets, capability policy, an audit chain, and a kill
# switch. Users get autonomous swarms/ensembles with absolute control.

substrate_rag = RagPipeline()          # process-lifetime RAG store
substrate_kills: dict[str, object] = {}  # run_id -> Governor (for the kill switch)


async def _substrate_llm(model_id: str, messages: list[dict], system: str | None) -> str:
    if not router:
        raise RuntimeError("Router not initialized")
    req = ChatRequest(
        model_id=model_id,
        messages=[ChatMessage(role=m["role"], content=m["content"]) for m in messages],
        system=system,
    )
    resp = await router.chat(req)
    return resp.content


def _make_governor(budget: dict | None, policy: dict | None) -> Governor:
    b = Budget(**budget) if budget else Budget()
    p = CapabilityPolicy(**policy) if policy else CapabilityPolicy()
    gov = Governor(budget=b, policy=p)
    if grpc:
        # One id per run, threaded through as the kernel event's session_id
        # so every event this Governor's audit chain produces (swarm_start,
        # agent:*, budget_exceeded, swarm_end, ...) can be correlated back to
        # a single run in the kernel's event store — previously always "".
        run_id = str(uuid.uuid4())
        def _mirror_to_kernel(kind: str, payload: dict) -> None:
            # Fire-and-forget — same graceful-degradation contract as every
            # other `grpc.*` call in this file (kernel absent == silently skipped).
            try:
                asyncio.create_task(grpc.append_event("orchestrator-substrate", kind, payload, run_id))
            except Exception:
                pass
        gov.audit.attach_kernel_mirror(_mirror_to_kernel)
    return gov


class SwarmReq(BaseModel):
    topology: str                        = "orchestrator"   # pipeline|parallel|orchestrator|debate
    task:     str
    agents:   list[dict]                 = []               # {id,name,system,model,role,temperature}
    rounds:   int                        = 2
    budget:   dict | None                = None
    policy:   dict | None                = None

class EnsembleReq(BaseModel):
    prompt:      str
    models:      list[str]
    system:      str | None              = None
    strategy:    str                     = "judge"          # concat|vote|judge|moa
    judge_model: str | None              = None
    budget:      dict | None             = None
    policy:      dict | None             = None

class RagIngestReq(BaseModel):
    doc_id:   str
    text:     str
    metadata: dict                       = {}

class RagQueryReq(BaseModel):
    query:  str
    k:      int                          = 5
    doc_id: str | None                   = None

class DistillReq(BaseModel):
    prompts:      list[str]
    teachers:     list[str]
    judge_model:  str | None             = None
    system:       str | None             = None
    backend:      str | None             = None             # unsloth|axolotl|llama.cpp|mlx
    base_model:   str                    = ""


@app.post("/api/swarm/run")
async def swarm_run(req: SwarmReq):
    if not router:
        raise HTTPException(503, "Router not initialized")
    gov = _make_governor(req.budget, req.policy)
    agents = [
        AgentSpec(
            id=a.get("id", f"agent{i}"), name=a.get("name", f"Agent {i}"),
            system=a.get("system", "You are a helpful expert."),
            model=a.get("model") or (router.list_providers() and f"{router.list_providers()[0]}") or "anthropic/claude-sonnet-4-6",
            temperature=float(a.get("temperature", 0.3)), role=a.get("role", "worker"),
        )
        for i, a in enumerate(req.agents)
    ]
    if not agents:
        raise HTTPException(400, "at least one agent is required")
    coord = SwarmCoordinator(_substrate_llm, gov)
    try:
        result = await coord.run(req.topology, agents, req.task, rounds=req.rounds)
    except Exception as exc:  # noqa: BLE001 — surface governance/limit errors cleanly
        raise HTTPException(400, str(exc))
    return {
        "output": result.output, "topology": result.topology,
        "steps": result.steps, "blackboard": result.blackboard,
        "governance": gov.report(),
    }


@app.post("/api/ensemble/run")
async def ensemble_run(req: EnsembleReq):
    if not router:
        raise HTTPException(503, "Router not initialized")
    gov = _make_governor(req.budget, req.policy)
    try:
        result = await run_ensemble(
            _substrate_llm, req.prompt, req.models, system=req.system,
            strategy=req.strategy, judge_model=req.judge_model, governor=gov,
        )
    except Exception as exc:  # noqa: BLE001
        raise HTTPException(400, str(exc))
    result["governance"] = gov.report()
    return result


@app.post("/api/rag/ingest")
async def rag_ingest(req: RagIngestReq):
    added = substrate_rag.ingest(req.doc_id, req.text, req.metadata)
    return {"doc_id": req.doc_id, "chunks_added": added, "stats": substrate_rag.stats()}


@app.post("/api/rag/query")
async def rag_query(req: RagQueryReq):
    hits = substrate_rag.retrieve(req.query, req.k, req.doc_id)
    return {"results": [{"doc_id": h.doc_id, "text": h.text, "score": h.score, "metadata": h.metadata} for h in hits]}


@app.post("/api/distill/build")
async def distill_build(req: DistillReq):
    if not router:
        raise HTTPException(503, "Router not initialized")
    builder = DistillationDatasetBuilder(_substrate_llm, req.teachers, req.judge_model, req.system)
    count = await builder.build(req.prompts)
    out = {"records": count, "dataset_jsonl": builder.to_jsonl()}
    if req.backend and req.base_model:
        out["training_config"] = builder.training_config(req.backend, req.base_model)
    return out

# ── WebSocket streaming chat ──────────────────────────────────────────────────

@app.websocket("/ws/chat/{session_id}")
async def ws_chat(websocket: WebSocket, session_id: str):
    await websocket.accept()
    try:
        while True:
            data = await websocket.receive_json()
            model_id = data.get("model_id", "claude-sonnet-4-6")
            content  = data.get("content", "")
            if not content:
                continue

            if episodic:
                await episodic.add_turn(session_id, "user", content)

            history = await episodic.get_history(session_id) if episodic else []
            chat_req = ChatRequest(
                model_id=model_id, messages=history,
                temperature=data.get("temperature", 0.7),
                max_tokens=data.get("max_tokens", 4096),
                system=data.get("system"),
                stream=True,
            )

            full_response = ""
            async for chunk in router.stream(chat_req):
                full_response += chunk
                await websocket.send_json({"delta": chunk, "done": False})

            if episodic:
                await episodic.add_turn(session_id, "assistant", full_response)

            await websocket.send_json({"delta": "", "done": True, "content": full_response})

    except WebSocketDisconnect:
        pass
    except Exception as e:
        try:
            await websocket.send_json({"error": str(e)})
        except Exception:
            pass


def create_app() -> FastAPI:
    return app


if __name__ == "__main__":
    import uvicorn
    uvicorn.run("omniharness.server:app", host="0.0.0.0", port=8080, reload=True)
