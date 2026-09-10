//! Módulo 05 — Orquestração
//! README: seção "Exercício" — Tolerância a falhas
//!
//! Estende o coordenador com detecção de falha via heartbeat/timeout e
//! reatribuição de shard. Se um worker não envia heartbeat em 5s, seu shard
//! é movido para o próximo worker vivo (round-robin).

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
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tonic::{Request, Response, Status, Streaming};
use tracing::{info, warn};

#[derive(Debug, Clone)]
struct WorkerInfo {
    shard_id: i32,
    last_heartbeat_ms: u128,
    alive: bool,
}

#[derive(Debug, Default)]
struct CoordinatorState {
    workers: HashMap<String, WorkerInfo>,
    files: Vec<String>,
    total_shards: usize,
    // Mapeia shard_id -> worker_id atual (para reatribuição)
    shard_owner: HashMap<i32, String>,
}

impl CoordinatorState {
    fn new(num_files: usize, total_shards: usize) -> Self {
        Self {
            files: gerar_lista_parquet(num_files),
            total_shards,
            ..Default::default()
        }
    }

    /// Verifica heartbeats e reatribui shards de workers mortos.
    /// Retorna lista de shards reatribuídos.
    fn check_heartbeats(&mut self, timeout_ms: u128) -> Vec<(i32, String, String)> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let mut dead = Vec::new();
        for (id, w) in self.workers.iter_mut() {
            if w.alive && now.saturating_sub(w.last_heartbeat_ms) > timeout_ms {
                w.alive = false;
                dead.push((w.shard_id, id.clone()));
                warn!(
                    "worker {id} considerado morto (sem heartbeat há {}ms)",
                    now - w.last_heartbeat_ms
                );
            }
        }
        let mut reatribuidos = Vec::new();
        for (shard_id, dead_id) in dead {
            // Encontra próximo worker vivo
            if let Some((novo_id, _)) = self
                .workers
                .iter()
                .find(|(id, w)| w.alive && *id != &dead_id)
            {
                let novo = novo_id.clone();
                // Atualiza shard_owner
                self.shard_owner.insert(shard_id, novo.clone());
                if let Some(_w) = self.workers.get_mut(&novo) {
                    // Worker vivo agora assume 2 shards — em produção, rebalancearia
                }
                reatribuidos.push((shard_id, dead_id.clone(), novo.clone()));
                info!("shard {shard_id} reatribuído de {dead_id} para {novo}");
            } else {
                warn!("nenhum worker vivo para reassumir shard {shard_id}");
            }
        }
        reatribuidos
    }
}

struct FaultTolerantService {
    state: Arc<Mutex<CoordinatorState>>,
}

#[tonic::async_trait]
impl Orchestrator for FaultTolerantService {
    async fn register_worker(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        let mut state = self.state.lock().unwrap();
        let shard_id = (state.workers.len() as i32) % state.total_shards as i32;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        state.workers.insert(
            req.worker_id.clone(),
            WorkerInfo {
                shard_id,
                last_heartbeat_ms: now,
                alive: true,
            },
        );
        state.shard_owner.insert(shard_id, req.worker_id.clone());
        info!("worker {} registrado shard {shard_id}", req.worker_id);
        Ok(Response::new(RegisterResponse {
            accepted: true,
            message: format!("shard {shard_id}"),
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
            w.alive = true;
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

        // Se este worker assumiu shard de outro morto, retorna também esses arquivos
        // Para demo, retorna apenas seu shard original + shards reatribuídos que ele herdou
        let mut shards: Vec<i32> = vec![worker.shard_id];
        for (shard, owner) in &state.shard_owner {
            if owner == &req.worker_id && *shard != worker.shard_id {
                shards.push(*shard);
            }
        }

        let mut files = Vec::new();
        for shard_id in shards {
            files.extend(
                state
                    .files
                    .iter()
                    .filter(|f| shard_for_file(f, state.total_shards) == shard_id as usize)
                    .cloned(),
            );
        }

        info!(
            "GetWork {} -> {} shards, {} arquivos",
            req.worker_id,
            worker.shard_id,
            files.len()
        );
        Ok(Response::new(GetWorkResponse {
            files,
            shard_id: worker.shard_id,
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
                "progresso {} file={} {}/{} status={}",
                req.worker_id, req.file, req.processed, req.total, req.status
            );
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
    let state_clone = state.clone();

    // Task de detecção de falhas a cada 2s, timeout 5s
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            let mut s = state_clone.lock().unwrap();
            let re = s.check_heartbeats(5000);
            if !re.is_empty() {
                info!("reatribuições: {:?}", re);
            }
        }
    });

    println!("Coordenador com tolerância a falhas em {addr}");
    println!("Timeout heartbeat: 5s, check a cada 2s");
    println!("Simule falha matando um worker: kill <pid> ou Ctrl+C");

    let svc = FaultTolerantService { state };

    tonic::transport::Server::builder()
        .add_service(OrchestratorServer::new(svc))
        .serve(addr)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_worker_morto() {
        let mut state = CoordinatorState::new(6, 2);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        state.workers.insert(
            "w1".to_string(),
            WorkerInfo {
                shard_id: 0,
                last_heartbeat_ms: now - 10000, // 10s atrás
                alive: true,
            },
        );
        state.workers.insert(
            "w2".to_string(),
            WorkerInfo {
                shard_id: 1,
                last_heartbeat_ms: now,
                alive: true,
            },
        );
        state.shard_owner.insert(0, "w1".to_string());
        state.shard_owner.insert(1, "w2".to_string());

        let re = state.check_heartbeats(5000);
        assert_eq!(re.len(), 1);
        assert_eq!(re[0].0, 0); // shard 0
        assert!(!state.workers["w1"].alive);
        assert!(state.workers["w2"].alive);
    }

    #[test]
    fn nao_reatribui_se_nenhum_vivo() {
        let mut state = CoordinatorState::new(3, 1);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        state.workers.insert(
            "w1".to_string(),
            WorkerInfo {
                shard_id: 0,
                last_heartbeat_ms: now - 10000,
                alive: true,
            },
        );
        state.shard_owner.insert(0, "w1".to_string());
        let re = state.check_heartbeats(5000);
        // w1 morto, mas sem outro vivo → nenhuma reatribuição
        assert!(re.is_empty());
    }

    #[test]
    fn heartbeat_atualiza_timestamp() {
        let mut state = CoordinatorState::new(3, 2);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        state.workers.insert(
            "w1".to_string(),
            WorkerInfo {
                shard_id: 0,
                last_heartbeat_ms: now - 1000,
                alive: true,
            },
        );
        // Simula heartbeat
        state.workers.get_mut("w1").unwrap().last_heartbeat_ms = now;
        let re = state.check_heartbeats(5000);
        assert!(re.is_empty());
        assert!(state.workers["w1"].alive);
    }
}
