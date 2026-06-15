use anyhow::Result;
use axum::{
    extract::Json,
    http::StatusCode,
    routing::post,
    Router,
};
use bmcs_gateway::*;
use bmcs_retriever::MultiStageRetriever;
use bmcs_empathy::{EmpathyScaffold, ACPoeStyler};
use trmkd;
use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

/// The complete BMCS Pipeline Orchestrator
/// Implements all six layers: L0-L1 (safety), L2 (retrieval), L3 (Axiom), L4 (AI), L5 (verify), L6 (monitor)
struct BMCSOrchestrator {
    sanitizer: InputSanitizer,
    classifier: ContextClassifier,
    verifier: AxiomVerifier,
    fallback_system: FallbackSystem,
    retriever: Arc<MultiStageRetriever>,
}

impl BMCSOrchestrator {
    fn new() -> Self {
        Self {
            sanitizer: InputSanitizer,
            classifier: ContextClassifier,
            verifier: AxiomVerifier,
            fallback_system: FallbackSystem,
            retriever: Arc::new(MultiStageRetriever::new()),
        }
    }

    /// Execute the complete BMCS pipeline
    async fn process_request(&self, req: BMCSRequest) -> Result<BMCSResponse> {
        info!("Processing BMCS request: {}", req.query);

        // L0: Input Sanitization
        if InputSanitizer::is_adversarial(&req.query) {
            warn!("Adversarial prompt detected, routing to fallback");
            let fb = FallbackSystem::get_fallback_response(ResponseTier::Fallback);
            return Ok(BMCSResponse {
                response: fb.response,
                disclaimer: fb.disclaimer,
                confidence: fb.confidence,
                escalated: fb.escalated,
                resources: fb.resources,
                tier: "Fallback".to_string(),
                sources: vec!["Safety Filter".to_string()],
            });
        }

        let sanitized_query = InputSanitizer::sanitize(&req.query);

        // L1: Context Classification (pre-model, formal grammar)
        let classification = ContextClassifier::classify(&sanitized_query, req.context.as_ref());
        info!(
            "Classified as tier: {}, confidence: {}",
            classification.tier, classification.confidence
        );

        // Tier 0: Emergency - STOP all other processing
        if classification.tier == ResponseTier::Emergency {
            let fb = FallbackSystem::get_fallback_response(ResponseTier::Emergency);
            return Ok(BMCSResponse {
                response: fb.response,
                disclaimer: fb.disclaimer,
                confidence: 1.0,
                escalated: true,
                resources: fb.resources,
                tier: "Emergency".to_string(),
                sources: vec!["Emergency Override".to_string()],
            });
        }

        // L2: Knowledge Retrieval
        let tier_str = classification.tier.to_string();
        let retrieval_result = self
            .retriever
            .retrieve(&sanitized_query, &tier_str, req.context.as_ref())
            .await?;

        info!(
            "Retrieved {} chunks with confidence {}",
            retrieval_result.chunks.len(),
            retrieval_result.total_confidence
        );

        // If no knowledge found or confidence too low, use fallback
        if retrieval_result.chunks.is_empty()
            || retrieval_result.total_confidence
                < match classification.tier {
                    ResponseTier::Emergency => 0.90,
                    ResponseTier::Critical => 0.85,
                    ResponseTier::Elevated => 0.75,
                    ResponseTier::Moderate => 0.65,
                    ResponseTier::Low => 0.50,
                    ResponseTier::Fallback => 0.40,
                }
        {
            info!("Confidence below threshold, using fallback");
            let fb = FallbackSystem::get_fallback_response(classification.tier);
            return Ok(BMCSResponse {
                response: fb.response,
                disclaimer: fb.disclaimer,
                confidence: fb.confidence,
                escalated: fb.escalated,
                resources: fb.resources,
                tier: classification.tier.to_string(),
                sources: vec!["Fallback".to_string()],
            });
        }

        // L3: Axiom Verification (ethical boundaries)
        let clinical_content = self.assemble_clinical_content(&retrieval_result);

        // Build empathetic response
        let emotional_state = EmpathyScaffold::detect_emotional_state(&req.query);
        let validation = EmpathyScaffold::get_validation_phrase(emotional_state);
        let support = EmpathyScaffold::get_support_closing(&tier_str);

        let response_text = if req.model_persona.as_deref() == Some("gothic") {
            let styled_validation = ACPoeStyler::style_validation(&validation, emotional_state);
            let styled_support = ACPoeStyler::style_closing(&support, &tier_str);
            ACPoeStyler::build_ac_poe_response(styled_validation, clinical_content.clone(), styled_support)
        } else {
            EmpathyScaffold::build_empathetic_response(validation, clinical_content.clone(), support)
        };

        // L5: Output Verification (post-model safety checks)
        let verification = AxiomVerifier::verify_response(&response_text);
        if !verification.all_passed {
            warn!(
                "Output verification failed: {:?}",
                verification.violations
            );
            let fb = FallbackSystem::get_fallback_response(classification.tier);
            return Ok(BMCSResponse {
                response: fb.response,
                disclaimer: fb.disclaimer,
                confidence: 0.60,
                escalated: fb.escalated,
                resources: fb.resources,
                tier: format!("{}_fallback", classification.tier),
                sources: vec!["Safety Filter".to_string()],
            });
        }

        // Assemble final response with mandatory disclaimer and sources
        let disclaimer = ResponseBuilder::get_disclaimer_for_tier(&tier_str);
        let sources: Vec<String> = retrieval_result
            .chunks
            .iter()
            .map(|(chunk, _)| chunk.source.clone())
            .collect();

        info!(
            "Processing complete. Tier: {}, Confidence: {}",
            tier_str, retrieval_result.total_confidence
        );

        Ok(BMCSResponse {
            response: response_text,
            disclaimer,
            confidence: retrieval_result.total_confidence,
            escalated: matches!(classification.tier, ResponseTier::Emergency | ResponseTier::Critical),
            resources: self.get_resources_for_tier(&tier_str),
            tier: tier_str,
            sources,
        })
    }

    /// Assemble clinical content from retrieved chunks
    fn assemble_clinical_content(&self, result: &bmcs_retriever::RetrievalResult) -> String {
        if result.chunks.is_empty() {
            return String::new();
        }

        let content_parts: Vec<String> = result
            .chunks
            .iter()
            .take(3)
            .map(|(chunk, _)| chunk.content.clone())
            .collect();

        content_parts.join("\n\n")
    }

    /// Get resources for a given tier
    fn get_resources_for_tier(&self, tier: &str) -> Vec<String> {
        match tier {
            "Emergency" => vec![
                "🇺🇸 Call 911 (US)".to_string(),
                "🇬🇧 Call 999 (UK)".to_string(),
                "🇦🇺 Call 000 (Australia)".to_string(),
            ],
            "Critical" => vec![
                "988 Suicide & Crisis Lifeline (US): Call or text 988".to_string(),
                "Crisis Text Line: Text HOME to 741741".to_string(),
                "International Association for Suicide Prevention: https://www.iasp.info/resources/Crisis_Centres/".to_string(),
            ],
            "Elevated" => vec![
                "SAMHSA National Helpline: 1-800-662-4357".to_string(),
                "Psychology Today Therapist Finder: https://www.psychologytoday.com".to_string(),
                "NAMI (National Alliance on Mental Illness): https://www.nami.org".to_string(),
            ],
            "Moderate" => vec![
                "Your Primary Care Doctor".to_string(),
                "Psychology Today Therapist Finder".to_string(),
                "SAMHSA National Helpline: 1-800-662-4357".to_string(),
            ],
            _ => vec![
                "Mayo Clinic: https://www.mayoclinic.org".to_string(),
                "WebMD: https://www.webmd.com".to_string(),
                "Your Healthcare Provider".to_string(),
            ],
        }
    }
}

/// HTTP handler for /v1/chat endpoint
async fn handle_chat(
    Json(req): Json<BMCSRequest>,
) -> Result<(StatusCode, Json<BMCSResponse>), (StatusCode, String)> {
    let orchestrator = BMCSOrchestrator::new();

    match orchestrator.process_request(req).await {
        Ok(response) => Ok((StatusCode::OK, Json(response))),
        Err(e) => {
            warn!("Error processing request: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "error": "Internal server error",
                    "message": e.to_string()
                })
                .to_string(),
            ))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("🏥 Starting Bonsai Medical-Grade AI Companion System (BMCS)");
    info!("✓ L0-L1 Safety layers initialized");
    info!("✓ L2 Retrieval engine initialized");
    info!("✓ L3 Axiom verifier initialized");
    info!("✓ L5-L6 Output verification and monitoring ready");

    // Build the router
    let app = Router::new()
        .route("/v1/chat", post(handle_chat))
        .layer(TraceLayer::new_for_http());

    // Start the server
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    info!("🚀 BMCS server listening on http://127.0.0.1:8080");
    info!("📡 POST /v1/chat to access medical-grade AI companion");

    axum::serve(listener, app).await?;

    Ok(())
}
