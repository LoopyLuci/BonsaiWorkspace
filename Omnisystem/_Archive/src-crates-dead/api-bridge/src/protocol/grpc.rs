use std::net::SocketAddr;
use std::sync::Arc;

use tonic::{transport::Server, Request, Response, Status};

use crate::auth::capability::CapabilityToken;
use crate::gateway::{dispatch_grpc_request, BridgeState};

pub mod proto {
    tonic::include_proto!("bonsai.api.bridge");
}

use proto::bridge_service_server::{BridgeService, BridgeServiceServer};
use proto::{BridgeJsonRequest, BridgeJsonResponse};

#[derive(Clone)]
pub struct BridgeGrpcService {
    pub state: Arc<BridgeState>,
}

#[tonic::async_trait]
impl BridgeService for BridgeGrpcService {
    async fn chat_completion(
        &self,
        request: Request<BridgeJsonRequest>,
    ) -> Result<Response<BridgeJsonResponse>, Status> {
        let inner = request.into_inner();
        let token: CapabilityToken = serde_json::from_str(&inner.authorization)
            .map_err(|e| Status::unauthenticated(e.to_string()))?;
        token.verify().map_err(|e| Status::unauthenticated(e.to_string()))?;

        let payload: serde_json::Value = serde_json::from_str(&inner.payload_json)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let response = dispatch_grpc_request(
            self.state.clone(),
            "/api/v1/chat/completions",
            payload,
            token,
        )
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(BridgeJsonResponse {
            status_code: 200,
            body_json: serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string()),
        }))
    }

    async fn list_peers(
        &self,
        request: Request<BridgeJsonRequest>,
    ) -> Result<Response<BridgeJsonResponse>, Status> {
        let inner = request.into_inner();
        let token: CapabilityToken = serde_json::from_str(&inner.authorization)
            .map_err(|e| Status::unauthenticated(e.to_string()))?;
        token.verify().map_err(|e| Status::unauthenticated(e.to_string()))?;

        let response = dispatch_grpc_request(
            self.state.clone(),
            "/api/v1/remote/peers",
            serde_json::json!({}),
            token,
        )
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(BridgeJsonResponse {
            status_code: 200,
            body_json: serde_json::to_string(&response).unwrap_or_else(|_| "{}".to_string()),
        }))
    }
}

pub async fn run_grpc_gateway(host: &str, port: u16, state: Arc<BridgeState>) -> anyhow::Result<()> {
    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    let service = BridgeGrpcService { state };
    Server::builder()
        .add_service(BridgeServiceServer::new(service))
        .serve(addr)
        .await?;
    Ok(())
}
