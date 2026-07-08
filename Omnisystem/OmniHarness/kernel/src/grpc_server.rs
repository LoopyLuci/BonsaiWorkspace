use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tonic::service::Interceptor;
use tonic::{transport::Server, Request, Response, Status};

use crate::{
    auth::AuthStore,
    event_store::{PersistentEventStore, SystemEvent},
    model_router::ModelRegistry,
    sandbox::Sandbox,
    session_store::SessionStore,
    tool_registry::ToolRegistry,
    vector_store::VectorStore,
};

// Import generated proto types
pub mod proto {
    tonic::include_proto!("omniharness.v1");
}

use proto::{
    event_store_service_server::{EventStoreService, EventStoreServiceServer},
    harness_service_server::{HarnessService, HarnessServiceServer},
    memory_service_server::{MemoryService, MemoryServiceServer},
    model_service_server::{ModelService, ModelServiceServer},
    session_service_server::{SessionService, SessionServiceServer},
    tool_service_server::{ToolService, ToolServiceServer},
    *,
};

// ── Shared state ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct HarnessState {
    pub event_store:    Arc<PersistentEventStore>,
    pub model_registry: Arc<ModelRegistry>,
    pub vector_store:   Arc<VectorStore>,
    pub session_store:  Arc<SessionStore>,
    pub tool_registry:  Arc<ToolRegistry>,
    pub auth_store:     Arc<AuthStore>,
    pub sandbox:        Arc<Sandbox>,
    pub start_time:     Instant,
}

// ── EventStore ────────────────────────────────────────────────────────────────

struct EventStoreSvc(Arc<HarnessState>);

#[tonic::async_trait]
impl EventStoreService for EventStoreSvc {
    async fn append_event(&self, req: Request<AppendRequest>) -> Result<Response<AppendResponse>, Status> {
        let r = req.into_inner();
        match self.0.event_store.append_event(&r.module_source, &r.event_type, &r.payload_json, &r.session_id).await {
            Ok(ev) => Ok(Response::new(AppendResponse {
                event_hash: ev.current_hash,
                event_id:   ev.id,
                success:    true,
                error:      String::new(),
            })),
            Err(e) => Ok(Response::new(AppendResponse {
                event_hash: String::new(), event_id: String::new(),
                success: false, error: e.to_string(),
            })),
        }
    }

    async fn verify_chain(&self, _: Request<VerifyRequest>) -> Result<Response<VerifyResponse>, Status> {
        match self.0.event_store.verify_chain().await {
            Ok(_) => Ok(Response::new(VerifyResponse {
                is_valid: true,
                tip_hash: self.0.event_store.current_tip().await,
                depth:    self.0.event_store.chain_depth().await,
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    type QueryEventsStream = tokio_stream::wrappers::ReceiverStream<Result<proto::SystemEvent, Status>>;

    async fn query_events(&self, req: Request<QueryRequest>) -> Result<Response<Self::QueryEventsStream>, Status> {
        let r = req.into_inner();
        let events = self.0.event_store
            .query_events(&r.module_source, &r.event_type, r.since_ts, r.limit as usize)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let (tx, rx) = tokio::sync::mpsc::channel(128);
        tokio::spawn(async move {
            for ev in events {
                let proto_ev = proto::SystemEvent {
                    id:            ev.id,
                    timestamp_utc: ev.timestamp_utc,
                    module_source: ev.module_source,
                    event_type:    ev.event_type,
                    payload_json:  ev.payload_json,
                    previous_hash: ev.previous_hash,
                    current_hash:  ev.current_hash,
                    session_id:    ev.session_id,
                };
                if tx.send(Ok(proto_ev)).await.is_err() { break; }
            }
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn get_tip(&self, _: Request<TipRequest>) -> Result<Response<TipResponse>, Status> {
        Ok(Response::new(TipResponse {
            tip_hash: self.0.event_store.current_tip().await,
            depth:    self.0.event_store.chain_depth().await,
        }))
    }
}

// ── Model Service ──────────────────────────────────────────────────────────────

struct ModelSvc(Arc<HarnessState>);

#[tonic::async_trait]
impl ModelService for ModelSvc {
    async fn chat(&self, req: Request<ChatRequest>) -> Result<Response<ChatResponse>, Status> {
        let r = req.into_inner();
        let chat_req = crate::model_router::ChatRequest {
            model_id:    r.model_id.clone(),
            messages:    r.messages.iter().map(|m| crate::model_router::ChatMessage {
                role:         m.role.clone(),
                content:      m.content.clone(),
                name:         if m.name.is_empty() { None } else { Some(m.name.clone()) },
                tool_call_id: if m.tool_call_id.is_empty() { None } else { Some(m.tool_call_id.clone()) },
            }).collect(),
            temperature: r.temperature,
            max_tokens:  r.max_tokens,
            system:      if r.system.is_empty() { None } else { Some(r.system) },
            tools:       r.tools.iter().map(|t| crate::model_router::ToolDef {
                name: t.name.clone(), description: t.description.clone(), schema: t.input_schema.clone(),
            }).collect(),
            session_id:  r.session_id,
        };

        match self.0.model_registry.route_chat(chat_req).await {
            Ok(resp) => {
                self.0.event_store.append_event(
                    "model", "ChatComplete",
                    &format!(r#"{{"model":"{}","tokens_out":{}}}"#, resp.model_used, resp.output_tokens),
                    "",
                ).await.ok();
                Ok(Response::new(ChatResponse {
                    content:       resp.content,
                    model_used:    resp.model_used,
                    finish_reason: resp.finish_reason,
                    input_tokens:  resp.input_tokens,
                    output_tokens: resp.output_tokens,
                    tool_calls:    resp.tool_calls.iter().map(|tc| proto::ToolCall {
                        id: tc.id.clone(), name: tc.name.clone(), arguments: tc.arguments.clone(),
                    }).collect(),
                    session_id:   String::new(),
                    latency_ms:   resp.latency_ms,
                }))
            }
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    type ChatStreamStream = tokio_stream::wrappers::ReceiverStream<Result<ChatChunk, Status>>;

    async fn chat_stream(&self, req: Request<ChatRequest>) -> Result<Response<Self::ChatStreamStream>, Status> {
        // Delegate to non-streaming and send as single chunk for now
        let resp = self.chat(req).await?.into_inner();
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(async move {
            tx.send(Ok(ChatChunk {
                content:       resp.content,
                is_final:      true,
                finish_reason: resp.finish_reason,
                output_tokens: resp.output_tokens,
            })).await.ok();
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn list_models(&self, req: Request<ListModelsRequest>) -> Result<Response<ListModelsResponse>, Status> {
        let filter = req.into_inner().provider;
        let models = self.0.model_registry.list_known_models()
            .into_iter()
            .filter(|m| filter.is_empty() || m.provider == filter)
            .map(|m| proto::ModelInfo {
                id:              m.id,
                provider:        m.provider,
                display_name:    m.display_name,
                context_window:  m.context_window,
                supports_tools:  m.supports_tools,
                supports_vision: m.supports_vision,
                available:       m.available,
                cost_per_1k_input:  0.0,
                cost_per_1k_output: 0.0,
            })
            .collect();
        Ok(Response::new(ListModelsResponse { models }))
    }

    async fn health_check(&self, req: Request<ModelHealthRequest>) -> Result<Response<ModelHealthResponse>, Status> {
        let provider = req.into_inner().provider;
        match self.0.model_registry.health_check(&provider).await {
            Ok(ms) => Ok(Response::new(ModelHealthResponse {
                healthy: true, provider, latency_ms: ms, error: String::new(),
            })),
            Err(e) => Ok(Response::new(ModelHealthResponse {
                healthy: false, provider, latency_ms: 0.0, error: e.to_string(),
            })),
        }
    }

    async fn register_model(&self, req: Request<RegisterModelReq>) -> Result<Response<RegisterModelResp>, Status> {
        let r = req.into_inner();
        self.0.model_registry.register(&r.provider, crate::model_router::ModelBackend {
            provider: r.provider.clone(),
            api_key:  r.api_key,
            base_url: r.base_url,
            params:   r.params.into_iter().collect(),
        });
        Ok(Response::new(RegisterModelResp { success: true, error: String::new() }))
    }
}

// ── Memory Service ─────────────────────────────────────────────────────────────

struct MemorySvc(Arc<HarnessState>);

#[tonic::async_trait]
impl MemoryService for MemorySvc {
    async fn store(&self, req: Request<StoreRequest>) -> Result<Response<StoreResponse>, Status> {
        let r = req.into_inner();
        let emb = if r.embed {
            None // vector store will auto-embed
        } else {
            None
        };
        match self.0.vector_store.store(&r.collection, &r.content, r.metadata.into_iter().collect(), emb).await {
            Ok(id) => Ok(Response::new(StoreResponse { id, success: true, error: String::new() })),
            Err(e) => Ok(Response::new(StoreResponse { id: String::new(), success: false, error: e.to_string() })),
        }
    }

    async fn retrieve(&self, req: Request<RetrieveRequest>) -> Result<Response<RetrieveResponse>, Status> {
        let r = req.into_inner();
        match self.0.vector_store.retrieve(&r.collection, &r.id) {
            Some(e) => Ok(Response::new(RetrieveResponse {
                entry: Some(proto::MemoryEntry {
                    id: e.id, collection: e.collection, content: e.content,
                    metadata: e.metadata, embedding: e.embedding,
                    created_at: e.created_at, score: 1.0,
                }),
                found: true,
            })),
            None => Ok(Response::new(RetrieveResponse { entry: None, found: false })),
        }
    }

    async fn search_semantic(&self, req: Request<SemanticSearchRequest>) -> Result<Response<SemanticSearchResponse>, Status> {
        let r       = req.into_inner();
        let top_k   = if r.top_k == 0 { 5 } else { r.top_k as usize };
        let thresh  = if r.threshold == 0.0 { 0.0 } else { r.threshold };
        let results = self.0.vector_store.search_semantic(&r.collection, &r.query, top_k, thresh);
        Ok(Response::new(SemanticSearchResponse {
            results: results.into_iter().map(|(e, score)| proto::MemoryEntry {
                id: e.id, collection: e.collection, content: e.content,
                metadata: e.metadata, embedding: e.embedding,
                created_at: e.created_at, score: score as f64,
            }).collect(),
        }))
    }

    async fn delete(&self, req: Request<DeleteRequest>) -> Result<Response<DeleteResponse>, Status> {
        let r = req.into_inner();
        let ok = self.0.vector_store.delete(&r.collection, &r.id);
        Ok(Response::new(DeleteResponse { success: ok, error: if ok { String::new() } else { "Not found".into() } }))
    }

    async fn list_collections(&self, _: Request<ListCollRequest>) -> Result<Response<ListCollResponse>, Status> {
        Ok(Response::new(ListCollResponse { collections: self.0.vector_store.list_collections() }))
    }

    async fn summarize(&self, req: Request<SummarizeRequest>) -> Result<Response<SummarizeResponse>, Status> {
        let r       = req.into_inner();
        let entries = self.0.vector_store.search_semantic(&r.collection, "", 100, 0.0);
        let combined: String = entries.iter().map(|(e, _)| e.content.as_str()).collect::<Vec<_>>().join("\n---\n");
        let summary = if combined.len() > r.max_tokens as usize * 4 {
            combined[..r.max_tokens as usize * 4].to_string()
        } else {
            combined
        };
        Ok(Response::new(SummarizeResponse { summary }))
    }
}

// ── Tool Service ───────────────────────────────────────────────────────────────

struct ToolSvc(Arc<HarnessState>);

#[tonic::async_trait]
impl ToolService for ToolSvc {
    async fn execute(&self, req: Request<ToolExecuteRequest>) -> Result<Response<ToolExecuteResponse>, Status> {
        let r = req.into_inner();
        match self.0.tool_registry.execute(&r.name, &r.arguments, r.timeout_ms as u32).await {
            Ok(tr) => Ok(Response::new(ToolExecuteResponse {
                result: tr.result, success: tr.success, error: String::new(), latency_ms: tr.latency_ms,
            })),
            Err(e) => Ok(Response::new(ToolExecuteResponse {
                result: String::new(), success: false, error: e.to_string(), latency_ms: 0.0,
            })),
        }
    }

    async fn register(&self, req: Request<ToolRegisterRequest>) -> Result<Response<ToolRegisterResponse>, Status> {
        let r = req.into_inner();
        self.0.tool_registry.register(crate::tool_registry::ToolDef {
            name: r.name, description: r.description, schema: r.schema,
            handler_url: if r.handler_url.is_empty() { None } else { Some(r.handler_url) },
            builtin: r.builtin,
        });
        Ok(Response::new(ToolRegisterResponse { success: true, error: String::new() }))
    }

    async fn list(&self, req: Request<ToolListRequest>) -> Result<Response<ToolListResponse>, Status> {
        let tools = self.0.tool_registry.list(req.into_inner().builtins_only)
            .into_iter().map(|t| proto::ToolDef {
                name: t.name, description: t.description, input_schema: t.schema,
            }).collect();
        Ok(Response::new(ToolListResponse { tools }))
    }

    async fn unregister(&self, req: Request<ToolUnregRequest>) -> Result<Response<ToolUnregResponse>, Status> {
        let ok = self.0.tool_registry.unregister(&req.into_inner().name);
        Ok(Response::new(ToolUnregResponse { success: ok, error: if ok { String::new() } else { "Not found".into() } }))
    }
}

// ── Session Service ────────────────────────────────────────────────────────────

struct SessionSvc(Arc<HarnessState>);

#[tonic::async_trait]
impl SessionService for SessionSvc {
    async fn create_session(&self, req: Request<CreateSessionReq>) -> Result<Response<CreateSessionResp>, Status> {
        let r = req.into_inner();
        match self.0.session_store.create(&r.title, &r.model_id, r.metadata.into_iter().collect()).await {
            Ok(s) => Ok(Response::new(CreateSessionResp {
                session: Some(proto::Session {
                    id: s.id, title: s.title, model_id: s.model_id,
                    created_at: s.created_at, updated_at: s.updated_at,
                    message_count: s.history.len() as i32, metadata: s.metadata,
                }),
                error: String::new(),
            })),
            Err(e) => Ok(Response::new(CreateSessionResp { session: None, error: e.to_string() })),
        }
    }

    async fn get_session(&self, req: Request<GetSessionReq>) -> Result<Response<GetSessionResp>, Status> {
        match self.0.session_store.get(&req.into_inner().id) {
            Some(s) => Ok(Response::new(GetSessionResp {
                session: Some(proto::Session {
                    id: s.id.clone(), title: s.title.clone(), model_id: s.model_id.clone(),
                    created_at: s.created_at, updated_at: s.updated_at,
                    message_count: s.history.len() as i32, metadata: s.metadata.clone(),
                }),
                history: s.history.iter().map(|m| proto::ChatMessage {
                    role: m.role.clone(), content: m.content.clone(),
                    name: m.name.clone().unwrap_or_default(),
                    tool_call_id: m.tool_call_id.clone().unwrap_or_default(),
                }).collect(),
                found: true,
            })),
            None => Ok(Response::new(GetSessionResp { session: None, history: vec![], found: false })),
        }
    }

    async fn list_sessions(&self, req: Request<ListSessionsReq>) -> Result<Response<ListSessionsResp>, Status> {
        let r = req.into_inner();
        let sessions = self.0.session_store.list(r.limit as usize, r.since)
            .into_iter().map(|s| proto::Session {
                id: s.id, title: s.title, model_id: s.model_id,
                created_at: s.created_at, updated_at: s.updated_at,
                message_count: s.history.len() as i32, metadata: s.metadata,
            }).collect();
        Ok(Response::new(ListSessionsResp { sessions }))
    }

    async fn delete_session(&self, req: Request<DeleteSessionReq>) -> Result<Response<DeleteSessionResp>, Status> {
        let ok = self.0.session_store.delete(&req.into_inner().id);
        Ok(Response::new(DeleteSessionResp { success: ok }))
    }
}

// ── Harness Service ────────────────────────────────────────────────────────────

struct HarnessSvc(Arc<HarnessState>);

#[tonic::async_trait]
impl HarnessService for HarnessSvc {
    async fn status(&self, _: Request<StatusRequest>) -> Result<Response<StatusResponse>, Status> {
        Ok(Response::new(StatusResponse {
            version:         "1.0.0".to_string(),
            healthy:         true,
            uptime_secs:     self.0.start_time.elapsed().as_secs() as i64,
            events_stored:   self.0.event_store.chain_depth().await,
            requests_served: self.0.event_store.request_count().await,
            tip_hash:        self.0.event_store.current_tip().await,
        }))
    }

    async fn config(&self, req: Request<ConfigRequest>) -> Result<Response<ConfigResponse>, Status> {
        let key = req.into_inner().key;
        let val = std::env::var(&key).ok();
        Ok(Response::new(ConfigResponse {
            value: val.clone().unwrap_or_default(),
            found: val.is_some(),
        }))
    }

    async fn reload(&self, _: Request<ReloadRequest>) -> Result<Response<ReloadResponse>, Status> {
        self.0.model_registry.register_from_env();
        Ok(Response::new(ReloadResponse { success: true, error: String::new() }))
    }

    async fn metrics(&self, _: Request<MetricsRequest>) -> Result<Response<MetricsResponse>, Status> {
        Ok(Response::new(MetricsResponse {
            avg_latency_ms:   0.0,
            p99_latency_ms:   0.0,
            total_tokens_in:  0,
            total_tokens_out: 0,
            cache_hits:       0,
            cache_misses:     0,
            provider_counts:  Default::default(),
        }))
    }
}

// ── Auth interceptor ────────────────────────────────────────────────────────
//
// `AuthStore` (auth.rs) has always generated and persisted an admin key on
// boot ("Set OMNIHARNESS_ADMIN_KEY env to use it") but nothing ever checked
// it — every gRPC call was accepted unconditionally. This closes that gap,
// opt-in via OMNIHARNESS_REQUIRE_AUTH so the default zero-config personal-use
// experience (a lone kernel on 127.0.0.1) is unaffected; set it to "1" for
// any deployment reachable by more than one trusted local process, e.g. a
// shared machine or a non-loopback GRPC_ADDR bind.

#[derive(Clone)]
struct AuthInterceptor {
    store: Arc<crate::auth::AuthStore>,
    required: bool,
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, req: Request<()>) -> Result<Request<()>, Status> {
        if !self.required {
            return Ok(req);
        }
        let key = req
            .metadata()
            .get("x-omniharness-key")
            .and_then(|v| v.to_str().ok());
        match key.and_then(|k| self.store.verify_key(k)) {
            Some(_) => Ok(req),
            None => Err(Status::unauthenticated(
                "missing or invalid x-omniharness-key metadata (OMNIHARNESS_REQUIRE_AUTH is set)",
            )),
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn serve(addr: SocketAddr, state: HarnessState) -> Result<()> {
    let required = std::env::var("OMNIHARNESS_REQUIRE_AUTH")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if required {
        tracing::info!("[Auth] Enforcing x-omniharness-key on every gRPC call (OMNIHARNESS_REQUIRE_AUTH set)");
    } else {
        tracing::info!("[Auth] Not enforced — default for local/personal use. Set OMNIHARNESS_REQUIRE_AUTH=1 to require the admin key.");
    }
    let interceptor = AuthInterceptor { store: Arc::clone(&state.auth_store), required };
    let state = Arc::new(state);
    Server::builder()
        .add_service(EventStoreServiceServer::with_interceptor(EventStoreSvc(Arc::clone(&state)), interceptor.clone()))
        .add_service(ModelServiceServer::with_interceptor(ModelSvc(Arc::clone(&state)), interceptor.clone()))
        .add_service(MemoryServiceServer::with_interceptor(MemorySvc(Arc::clone(&state)), interceptor.clone()))
        .add_service(ToolServiceServer::with_interceptor(ToolSvc(Arc::clone(&state)), interceptor.clone()))
        .add_service(SessionServiceServer::with_interceptor(SessionSvc(Arc::clone(&state)), interceptor.clone()))
        .add_service(HarnessServiceServer::with_interceptor(HarnessSvc(Arc::clone(&state)), interceptor))
        .serve(addr)
        .await?;
    Ok(())
}
