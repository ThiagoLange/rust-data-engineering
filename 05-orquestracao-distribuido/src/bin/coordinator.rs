//! Módulo 05 — Orquestração
//! README: seção "Exemplo prático: Worker distribuído" — Coordenador
//!
//! Coordenador gRPC que distribui arquivos Parquet via sharding por hash
//! e recebe progresso via streaming. Sem tolerância a falhas (ver exercício).

use anyhow::Result;
use orquestracao_distribuido::{
    gerar_lista_parquet,
    orchestrator::{
        orchestrator_server::{Orchestrator, OrchestratorServer},
        GetWorkRequest, GetWorkResponse, HeartbeatRequest, HeartbeatResponse, ProgressRequest,
        ProgressResponse, RegisterRequest, RegisterResponse,
    },
    shard_for_file,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status, Streaming};
use tracing::{info, warn};

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct WorkerInfo {
    addr: String,
    shard_id: i32,
    last_heartbeat_ms: u128,
}

#[derive(Debug, Default)]
struct CoordinatorState {
    workers: HashMap<String, WorkerInfo>,
    files: Vec<String>,
    total_shards: usize,
}

impl CoordinatorState {
    fn new(num_files: usize, total_shards: usize) -> Self {
        Self {
            files: gerar_lista_parquet(num_files),
            total_shards,
            ..Default::default()
        }
    }
}

struct OrchestratorService {
    state: Arc<Mutex<CoordinatorState>>,
}

#[tonic::async_trait]
impl Orchestrator for OrchestratorService {
    async fn register_worker(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        let mut state = self.state.lock().unwrap();
        // Atribui shard sequencialmente (round-robin)
        let shard_id = (state.workers.len() as i32) % state.total_shards as i32;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        state.workers.insert(
            req.worker_id.clone(),
            WorkerInfo {
                addr: req.addr.clone(),
                shard_id,
                last_heartbeat_ms: now,
            },
        );
        info!(
            "worker {} registrado (addr={}, shard={}/{}) — total workers={}",
            req.worker_id,
            req.addr,
            shard_id,
            state.total_shards,
            state.workers.len()
        );
        Ok(Response::new(RegisterResponse {
            accepted: true,
            message: format!("shard {shard_id} atribuído"),
        }))
    }

    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let req = request.into_inner();
        let mut state = self.state.lock().unwrap();
        if let Some(w) = state.workers.get_mut(&req.worker_id) {
            w.last_heartbeat_ms = req.timestamp_ms as u128;
        } else {
            warn!("heartbeat de worker desconhecido {}", req.worker_id);
        }
        Ok(Response::new(HeartbeatResponse { alive: true }))
    }

    async fn get_work(
        &self,
        request: Request<GetWorkRequest>,
    ) -> Result<Response<GetWorkResponse>, Status> {
        let req = request.into_inner();
        let state = self.state.lock().unwrap();
        let worker = state
            .workers
            .get(&req.worker_id)
            .ok_or_else(|| Status::not_found("worker não registrado"))?;
        let shard_id = worker.shard_id;
        let files: Vec<String> = state
            .files
            .iter()
            .filter(|f| shard_for_file(f, state.total_shards) == shard_id as usize)
            .cloned()
            .collect();
        info!(
            "GetWork worker={} shard={} → {} arquivos",
            req.worker_id,
            shard_id,
            files.len()
        );
        Ok(Response::new(GetWorkResponse {
            files,
            shard_id,
            total_shards: state.total_shards as i32,
        }))
    }

    async fn report_progress(
        &self,
        request: Request<Streaming<ProgressRequest>>,
    ) -> Result<Response<ProgressResponse>, Status> {
        let mut stream = request.into_inner();
        while let Some(req) = stream.message().await? {
            info!(
                "progresso worker={} file={} {}/{} status={}",
                req.worker_id, req.file, req.processed, req.total, req.status
            );
            if req.status == "error" {
                warn!("worker {} reportou erro em {}", req.worker_id, req.file);
            }
        }
        Ok(Response::new(ProgressResponse { ack: true }))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let addr = "127.0.0.1:50051".parse()?;
    let state = Arc::new(Mutex::new(CoordinatorState::new(12, 3)));
    println!("Coordenador escutando em {addr}");
    println!("Arquivos para processar: {:?}", state.lock().unwrap().files);
    println!("Aguardando workers: cargo run --bin worker -- --id 1");

    let svc = OrchestratorService { state };

    tonic::transport::Server::builder()
        .add_service(OrchestratorServer::new(svc))
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use orquestracao_distribuido::shard_files;

    #[test]
    fn sharding_por_hash_cobre_todos_arquivos() {
        let files = gerar_lista_parquet(9);
        let shards = shard_files(files.clone(), 3);
        let total: usize = shards.values().map(|v| v.len()).sum();
        assert_eq!(total, 9);
        // Cada shard do coordenador deve ter ~3 arquivos
        for (id, list) in shards {
            assert!(!list.is_empty(), "shard {id} vazio");
        }
    }
}
