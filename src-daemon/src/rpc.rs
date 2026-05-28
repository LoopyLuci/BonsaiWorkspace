use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use serde_json::Value;

use crate::state::DaemonState;

use bonsai_transfer_crypto::{
    identity::BonsaiIdentity,
    kdf::{kdf_phrase_to_seed, ARGON2_PARAMS_TEST},
    session::SessionKey,
};
use bonsai_transfer_core::{
    lane::{InProcessLane, TransportLane},
    scheduler::EcfRgScheduler,
    transfer::{Transfer, TransferStatus, TransferHandle},
};
use bonsai_array::AplEval;
use bonsai_verify_lean::{LeanSidecar, LeanRequest};
use bonsai_verify_coq::{CoqSidecar, CoqRequest};
use bonsai_verify_agda::{AgdaSidecar, AgdaRequest};
use bonsai_verify_isabelle::{IsabelleSidecar, IsabelleRequest};
use bonsai_capability_registry::{TrustScore, DeploymentGate};
use bonsai_sylva::SylvaVm;
use uuid::Uuid;
use tokio::sync::mpsc;
use std::collections::HashMap;
use bonsai_ci::{PipelineDef, OrchestratorActor, RunResult};

/// Dispatch a JSON-RPC method to a concrete implementation.
/// Returns a JSON value on success or an error string on failure.
pub async fn dispatch(
    method: &str,
    params: &Value,
    state: &Arc<DaemonState>,
) -> Result<Value, String> {
    match method {
        "identity.create" | "identity.create_identity" => {
            let phrase_opt = params.get("phrase").and_then(|v| v.as_str());
            let passphrase_opt = params.get("passphrase").and_then(|v| v.as_str());

            let identity = if let Some(phrase) = phrase_opt {
                let seed = kdf_phrase_to_seed(phrase, passphrase_opt, Some(ARGON2_PARAMS_TEST))
                    .map_err(|e| e.to_string())?;
                BonsaiIdentity::from_seed(&seed).map_err(|e| e.to_string())?
            } else {
                BonsaiIdentity::generate()
            };

            let payload = serde_json::json!({
                "seed": hex::encode(identity.export_seed()),
                "fingerprint": identity.fingerprint(),
            });

            state.store.save(&payload).map_err(|e| e.to_string())?;
            *state.identity.lock().await = Some(Arc::new(identity));

            // Return a compact DTO
            Ok(serde_json::json!({
                "fingerprint": payload["fingerprint"].as_str().unwrap_or("").to_string(),
                "public_key_hex": "",
            }))
        }

        "identity.unlock" | "identity.restore" => {
            let payload: serde_json::Value = state.store.load().map_err(|e| e.to_string())?;
            let seed_hex = payload["seed"].as_str().ok_or("missing seed in store")?;
            let seed_bytes = hex::decode(seed_hex).map_err(|e| e.to_string())?;
            if seed_bytes.len() != 32 { return Err("invalid seed length".into()); }
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&seed_bytes);
            let identity = BonsaiIdentity::from_seed(&seed).map_err(|e| e.to_string())?;
            let dto = serde_json::json!({
                "fingerprint": identity.fingerprint().to_string(),
                "public_key_hex": hex::encode(identity.public_key.to_hex()),
            });
            *state.identity.lock().await = Some(Arc::new(identity));
            Ok(dto)
        }

        "identity.get" | "identity.get_my_public_key" => {
            match state.identity.lock().await.as_ref() {
                Some(id) => Ok(serde_json::json!({
                    "fingerprint": id.fingerprint().to_string(),
                    "public_key_hex": hex::encode(id.public_key.to_hex()),
                })),
                None => Ok(serde_json::Value::Null),
            }
        }

        "contacts.list" | "contacts.list_contacts" => {
            // Load contacts from the encrypted store. If the store file is missing or
            // unreadable, treat it as an empty store and return an empty contacts array.
            let payload: serde_json::Value = match state.store.load() {
                Ok(v) => v,
                Err(_) => serde_json::json!({}),
            };
            let contacts = payload.get("contacts").cloned().unwrap_or_else(|| serde_json::json!([]));
            Ok(contacts)
        }

        "contacts.add_contact" => {
            // Expect a JSON object representing the contact.
            let contact = params.get("contact").ok_or("missing contact")?;
            // Load existing store JSON, insert/append contact, save back.
            let mut payload: serde_json::Value = match state.store.load() {
                Ok(v) => v,
                Err(_) => serde_json::json!({}),
            };
            let arr = payload.as_object_mut()
                .and_then(|o| o.get_mut("contacts"))
                .and_then(|v| v.as_array_mut());
            if let Some(a) = arr {
                a.push(contact.clone());
            } else {
                payload["contacts"] = serde_json::json!([contact.clone()]);
            }
            state.store.save(&payload).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({"ok": true}))
        }

        "transfer.send_file" => {
            let file_path = params.get("file_path").and_then(|v| v.as_str()).ok_or("missing file_path")?;
            let chunk_size = params.get("chunk_size").and_then(|v| v.as_u64()).map(|v| v as usize);

            let path = PathBuf::from(file_path);
            let data = tokio::fs::read(&path).await.map_err(|e| format!("read {file_path}: {e}"))?;

            // Get identity
            let guard = state.identity.lock().await;
            let identity = guard.as_ref().ok_or("no identity loaded")?.clone();
            drop(guard);

            // Derive a session key from the identity seed (loopback style)
            let seed = identity.export_seed();
            let key_bytes = blake3::derive_key("bonsai-loopback-session", &seed);
            let session_key = Arc::new(SessionKey(key_bytes));

            // Build in-process lanes + scheduler
            let (lane, _rx) = InProcessLane::new_pair("loopback");
            let mut lanes_map = std::collections::HashMap::new();
            let lane_arc: Arc<dyn bonsai_transfer_core::lane::TransportLane> = Arc::new(lane);
            lanes_map.insert("loopback".to_string(), lane_arc);
            let lanes = Arc::new(lanes_map);

            let mut sched = EcfRgScheduler::new();
            {
                let (lane2, _rx2) = InProcessLane::new_pair("loopback");
                sched.add_lane(Arc::new(lane2));
            }
            let scheduler = Arc::new(tokio::sync::Mutex::new(sched));

            let cs = chunk_size.unwrap_or(bonsai_transfer_core::transfer::DEFAULT_CHUNK_SIZE)
                .min(bonsai_transfer_core::transfer::MAX_CHUNK_SIZE)
                .max(1);

            let transfer = Transfer::new();

            // Create a progress channel so we can update DaemonState.transfers
            let (tx, mut rx) = mpsc::unbounded_channel();

            let handle = transfer.send_data(data.clone(), session_key, scheduler, lanes, cs, Some(tx)).await
                .map_err(|e| e.to_string())?;

            let id_str = handle.id.to_string();

            // Insert initial status and handle into state maps
            let initial_status = TransferStatus {
                id: handle.id,
                direction: bonsai_transfer_core::transfer::TransferDirection::Send,
                total_bytes: data.len() as u64,
                transferred_bytes: handle.bytes_sent(),
                chunk_count: (data.len().saturating_add(cs - 1) / cs) as u64,
                chunks_done: (handle.bytes_sent().saturating_add(cs as u64 - 1) / cs as u64),
                active_lanes: vec!["loopback".to_string()],
                state: bonsai_transfer_core::transfer::TransferState::Active,
                bytes_per_sec: 0.0,
            };

            state.transfers.lock().await.insert(id_str.clone(), initial_status.clone());
            state.transfer_handles.lock().await.insert(id_str.clone(), handle.clone());

            // Spawn a background task to watch progress updates and persist them in state
            let state_clone = state.clone();
            tokio::spawn(async move {
                while let Some(status) = rx.recv().await {
                    let key = status.id.to_string();
                    let mut map = state_clone.transfers.lock().await;
                    map.insert(key, status);
                }
            });

            // short sleep to let the transfer start
            tokio::time::sleep(Duration::from_millis(50)).await;

            // Return the initial DTO
            let dto = serde_json::json!({
                "id": id_str,
                "direction": "send",
                "total_bytes": data.len() as u64,
                "transferred_bytes": initial_status.transferred_bytes,
                "chunk_count": initial_status.chunk_count,
                "chunks_done": initial_status.chunks_done,
                "state": "active",
                "bytes_per_sec": 0.0,
                "progress_pct": initial_status.progress() * 100.0,
            });

            Ok(dto)
        }

        "transfer.list" | "transfer.list_transfers" => {
            let map = state.transfers.lock().await;
            let vals: Vec<TransferStatus> = map.values().cloned().collect();
            let out = serde_json::to_value(&vals).map_err(|e| e.to_string())?;
            Ok(out)
        }

        "transfer.cancel" | "transfer.cancel_transfer" => {
            let id = params.get("id").and_then(|v| v.as_str()).ok_or("missing id")?;
            let mut handles = state.transfer_handles.lock().await;
            if let Some(handle) = handles.get(id) {
                handle.cancel();
                // Update status map
                if let Some(mut status) = state.transfers.lock().await.get_mut(id) {
                    status.state = bonsai_transfer_core::transfer::TransferState::Cancelled;
                }
                Ok(serde_json::json!({"ok": true}))
            } else {
                Err("transfer handle not found".into())
            }
        }

        "data.execute_sql" => {
            let query = params.get("query").and_then(|v| v.as_str()).ok_or("missing query")?;
            let rows = state.sql.lock().await.query_json(query).map_err(|e| e.to_string())?;
            let out = serde_json::to_value(&rows).map_err(|e| e.to_string())?;
            Ok(out)
        }

        "data.eval_apl" | "data.eval" | "apl.eval" => {
            let src = params.get("src").or_else(|| params.get("code")).and_then(|v| v.as_str()).ok_or("missing src/code")?;
            let arr = AplEval::eval(src).map_err(|e| e.to_string())?;
            let out = serde_json::to_value(&arr).map_err(|e| e.to_string())?;
            Ok(out)
        }

        "deno.run_script" | "deno.eval" => {
            let code = params.get("code").and_then(|v| v.as_str()).ok_or("missing code")?;

            // Worker path relative to workspace
            let worker_path = PathBuf::from("runtimes/deno/worker.ts");
            if !worker_path.exists() {
                return Err(format!("deno worker not found: {}", worker_path.display()));
            }

            // Spawn deno subprocess
            let mut child = Command::new("deno")
                .arg("run")
                .arg("--quiet")
                .arg(worker_path)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("failed to spawn deno: {e}"))?;

            let mut stdin = child.stdin.take().ok_or("failed to open deno stdin")?;
            let stdout = child.stdout.take().ok_or("failed to open deno stdout")?;
            let mut reader = BufReader::new(stdout);

            // read startup line (timeout)
            let mut line = String::new();
            let _ = tokio::time::timeout(Duration::from_secs(3), reader.read_line(&mut line)).await
                .map_err(|_| "deno worker startup timed out" )?
                .map_err(|e: std::io::Error| e.to_string())?;

            // Send eval request
            let req_id = Uuid::new_v4().to_string();
            let req = serde_json::json!({"id": req_id, "op": "eval", "code": code});
            let req_line = serde_json::to_string(&req).map_err(|e| e.to_string())? + "\n";
            tokio::io::AsyncWriteExt::write_all(&mut stdin, req_line.as_bytes()).await.map_err(|e| e.to_string())?;

            // Read response lines until matching id or timeout
            let mut buf = String::new();
            let timeout = Duration::from_secs(10);
            let mut result_value: Option<serde_json::Value> = None;
            let start = tokio::time::Instant::now();
            while start.elapsed() < timeout {
                buf.clear();
                let n = tokio::time::timeout(timeout - start.elapsed(), reader.read_line(&mut buf)).await
                    .map_err(|_| "deno worker read timed out" )?
                    .map_err(|e: std::io::Error| e.to_string())?;
                if n == 0 { break; }
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(buf.trim()) {
                    if v.get("id").and_then(|x| x.as_str()) == Some(req_id.as_str()) {
                        result_value = Some(v);
                        break;
                    }
                }
            }

            // Try to shutdown the worker gracefully
            let _ = tokio::io::AsyncWriteExt::write_all(&mut stdin, serde_json::to_string(&serde_json::json!({"id": Uuid::new_v4().to_string(), "op": "shutdown"})).unwrap().as_bytes()).await;
            let _ = child.kill().await;

            let v = result_value.ok_or("no response from deno worker")?;
            if v.get("ok").and_then(|b| b.as_bool()) == Some(true) {
                Ok(v.get("result").cloned().unwrap_or(serde_json::Value::Null))
            } else {
                Err(v.get("error").and_then(|e| e.as_str()).unwrap_or("deno eval error").to_string())
            }
        }

        "sylva.eval" => {
            let src = params.get("src").or_else(|| params.get("code")).and_then(|v| v.as_str()).ok_or("missing src/code")?;
            // Wire tool_fn so Sylva scripts can call daemon RPC methods via
            // tool("method_name", args_json_value).
            let state_clone = state.clone();
            let tool_fn: bonsai_sylva::vm::ToolFn = Arc::new(move |method: String, args: serde_json::Value| {
                // Dispatch synchronously using a blocking runtime handle.
                let state2 = state_clone.clone();
                let method2 = method.clone();
                let result = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(async move {
                        dispatch(&method2, &args, &state2).await
                    })
                });
                result.map_err(|e| bonsai_sylva::VmError::ToolCallFailed(e))
            });
            let mut vm = SylvaVm::with_tool_fn(tool_fn);
            bonsai_sylva::stdlib::register_stdlib(&mut vm);
            let result = vm.eval_str(src).map_err(|e| e.to_string())?;
            Ok(result.to_json())
        }

        "verify.check_lean" | "verify.verify_lean" => {
            let src = params.get("source").or_else(|| params.get("src")).and_then(|v| v.as_str()).ok_or("missing source")?;
            let timeout_secs = params.get("timeout_secs").and_then(|v| v.as_u64()).map(|v| v as u64);
            let sidecar = LeanSidecar::new();
            let req = LeanRequest { source: src.to_string(), timeout_secs };
            match sidecar.verify(&req) {
                Ok(resp) => Ok(serde_json::to_value(&resp).map_err(|e| e.to_string())?),
                Err(e) => Err(e.to_string()),
            }
        }

        "verify.check_coq" | "verify.verify_coq" => {
            let src = params.get("source").or_else(|| params.get("src")).and_then(|v| v.as_str()).ok_or("missing source")?;
            let timeout_secs = params.get("timeout_secs").and_then(|v| v.as_u64());
            let req = CoqRequest { source: src.to_string(), logical_name: None, timeout_secs };
            let sidecar = CoqSidecar::new();
            match sidecar.verify(&req) {
                Ok(resp)  => Ok(serde_json::to_value(&resp).map_err(|e| e.to_string())?),
                Err(e)    => Err(e.to_string()),
            }
        }

        "verify.check_agda" | "verify.verify_agda" => {
            let src = params.get("source").or_else(|| params.get("src")).and_then(|v| v.as_str()).ok_or("missing source")?;
            let module = params.get("module_name").and_then(|v| v.as_str()).unwrap_or("BonsaiProof").to_string();
            let timeout_secs = params.get("timeout_secs").and_then(|v| v.as_u64());
            let req = AgdaRequest { source: src.to_string(), module_name: module, timeout_secs };
            let sidecar = AgdaSidecar::new();
            match sidecar.verify(&req) {
                Ok(resp)  => Ok(serde_json::to_value(&resp).map_err(|e| e.to_string())?),
                Err(e)    => Err(e.to_string()),
            }
        }

        "verify.check_isabelle" | "verify.verify_isabelle" => {
            let src = params.get("source").or_else(|| params.get("src")).and_then(|v| v.as_str()).ok_or("missing source")?;
            let theory = params.get("theory_name").and_then(|v| v.as_str()).unwrap_or("BonsaiProof").to_string();
            let imports = params.get("imports").and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let timeout_secs = params.get("timeout_secs").and_then(|v| v.as_u64());
            let req = IsabelleRequest { source: src.to_string(), theory_name: theory, imports, timeout_secs };
            let sidecar = IsabelleSidecar::new();
            match sidecar.verify(&req) {
                Ok(resp)  => Ok(serde_json::to_value(&resp).map_err(|e| e.to_string())?),
                Err(e)    => Err(e.to_string()),
            }
        }

        "ci.submit_pipeline" => {
            // Expect a `pipeline` object in params
            let pipeline_val = params.get("pipeline").ok_or("missing pipeline")?.clone();
            let pipeline: PipelineDef = serde_json::from_value(pipeline_val).map_err(|e| e.to_string())?;

            // Ensure orchestrator exists
            let mut orch_lock = state.orchestrator.lock().await;
            if orch_lock.is_none() {
                *orch_lock = Some(OrchestratorActor::new());
            }
            let orch = orch_lock.as_ref().unwrap();

            // Run Phase 1: execute first stage synchronously and return output
            match orch.submit_pipeline(pipeline).await {
                Ok(run) => {
                    let out = serde_json::json!({
                        "status": run.status,
                        "exit_code": run.exit_code,
                        "stdout": run.stdout,
                        "stderr": run.stderr,
                    });
                    Ok(out)
                }
                Err(e) => Err(e.to_string()),
            }
        }

        "tools.call" => {
            let name = params.get("name").and_then(|v| v.as_str()).ok_or("missing name")?;
            let args = params.get("args").cloned().unwrap_or(serde_json::Value::Null);
            let skill = state.tools.get(name).ok_or_else(|| format!("tool not found: {name}"))?;

            // ── Trust gate ──────────────────────────────────────────────────
            // Build a TrustScore from the skill's required permissions and
            // check it passes the Staging gate before executing.
            let mut score = TrustScore::default();
            use bonsai_capability_registry::effect_penalty;
            // Convert permission strings to BonsaiEffect for penalty calculation.
            // Unknown strings incur no penalty.
            let effects: Vec<bonsai_capability_registry::BonsaiEffect> = skill
                .requires_permissions.iter()
                .filter_map(|p| serde_json::from_value(serde_json::json!(p)).ok())
                .collect();
            score.add_capability_penalty(effect_penalty(&effects));
            let gate = DeploymentGate::Staging;
            if let bonsai_capability_registry::GateResult::Fail { required, actual, reason } = gate.check(&score) {
                return Err(format!("trust gate failed: score {actual} < required {required}: {reason}"));
            }
            // ── End trust gate ──────────────────────────────────────────────

            // Execute on a blocking thread — wasmtime is sync.
            let wasm_bytes = skill.wasm_bytes.clone();
            let result = tokio::task::spawn_blocking(move || {
                bonsai_skills::execute_skill(&wasm_bytes, &args)
            }).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;

            Ok(serde_json::to_value(&result).map_err(|e| e.to_string())?)
        }

        "tools.list" => {
            let defs = state.tools.list();
            Ok(serde_json::to_value(&defs).map_err(|e| e.to_string())?)
        }

        "tools.get" => {
            let name = params.get("name").and_then(|v| v.as_str()).ok_or("missing name")?;
            match state.tools.get(name) {
                Some(skill) => Ok(serde_json::json!({
                    "name": skill.name,
                    "description": skill.description,
                    "tags": skill.tags,
                    "requires_permissions": skill.requires_permissions,
                    "wasm_hash": skill.wasm_hash,
                    "rules": serde_json::to_value(&skill.rules).unwrap_or_default(),
                })),
                None => Err(format!("tool not found: {name}")),
            }
        }

        // ── P2P lanes ─────────────────────────────────────────────────────────

        "p2p.start_webrtc" => {
            let name  = params.get("name").and_then(|v| v.as_str()).unwrap_or("webrtc:default");
            let stuns: Vec<String> = params.get("stun_urls")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|s| s.as_str().map(String::from)).collect())
                .unwrap_or_else(|| vec!["stun:stun.l.google.com:19302".into()]);

            let (lane, offer_sdp) = bonsai_p2p::WebRtcLane::new_offer(name, stuns).await
                .map_err(|e| e.to_string())?;

            let lane_name = lane.name().to_string();
            state.webrtc_lanes.lock().await.insert(lane_name.clone(), lane.clone());
            state.p2p_lanes.lock().await.insert(lane_name.clone(), lane);
            Ok(serde_json::json!({ "lane": lane_name, "offer_sdp": offer_sdp }))
        }

        "p2p.accept_webrtc_answer" => {
            let name       = params.get("name").and_then(|v| v.as_str()).ok_or("missing name")?;
            let answer_sdp = params.get("answer_sdp").and_then(|v| v.as_str()).ok_or("missing answer_sdp")?;
            let lane = state.webrtc_lanes.lock().await
                .get(name)
                .cloned()
                .ok_or_else(|| format!("no WebRTC lane: {name}"))?;
            bonsai_p2p::WebRtcLane::accept_answer(&lane, answer_sdp).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true }))
        }

        "p2p.start_swarm" => {
            let name      = params.get("name").and_then(|v| v.as_str()).unwrap_or("swarm:default");
            let peer_addr = params.get("peer_addr").and_then(|v| v.as_str())
                .ok_or("missing peer_addr")?;
            let lane = bonsai_p2p::SwarmLane::connect(name, peer_addr).await
                .map_err(|e| e.to_string())?;
            let lane_name = lane.name().to_string();
            state.p2p_lanes.lock().await.insert(lane_name.clone(), lane);
            Ok(serde_json::json!({ "lane": lane_name }))
        }

        "p2p.start_onion" => {
            let name       = params.get("name").and_then(|v| v.as_str()).unwrap_or("onion:default");
            let target     = params.get("target").and_then(|v| v.as_str()).ok_or("missing target")?;
            let port       = params.get("port").and_then(|v| v.as_u64()).ok_or("missing port")? as u16;
            let proxy_addr = params.get("proxy_addr").and_then(|v| v.as_str())
                .unwrap_or("127.0.0.1:9050");
            let lane = bonsai_p2p::OnionLane::connect(name, proxy_addr, target, port).await
                .map_err(|e| e.to_string())?;
            let lane_name = lane.name().to_string();
            state.p2p_lanes.lock().await.insert(lane_name.clone(), lane);
            Ok(serde_json::json!({ "lane": lane_name }))
        }

        "p2p.list_lanes" => {
            let lanes = state.p2p_lanes.lock().await;
            let list: Vec<serde_json::Value> = lanes.values().map(|l| {
                let h = l.health();
                serde_json::json!({
                    "name":      l.name(),
                    "kind":      format!("{:?}", l.kind()),
                    "available": h.available,
                    "rtt_ms":    h.rtt_ms,
                    "bw_bps":    h.bandwidth_bps,
                })
            }).collect();
            Ok(serde_json::json!({ "lanes": list }))
        }

        "p2p.close_lane" => {
            let name = params.get("name").and_then(|v| v.as_str()).ok_or("missing name")?;
            let lane = state.p2p_lanes.lock().await.remove(name)
                .ok_or_else(|| format!("no lane: {name}"))?;
            lane.close().await;
            Ok(serde_json::json!({ "ok": true }))
        }

        "daemon.update_binary" => {
            let new_path = params.get("path").and_then(|v| v.as_str()).ok_or("missing path")?;
            let p = std::path::PathBuf::from(new_path);
            if !p.exists() { return Err(format!("binary not found: {new_path}")); }
            crate::binary_swap::replace_running_binary(&p).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "ok": true, "message": "binary replaced; re-exec to activate" }))
        }

        _ => Err(format!("unknown method: {}", method)),
    }
}
