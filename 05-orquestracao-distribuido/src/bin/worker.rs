//! Módulo 05 — Orquestração
//! README: seção "Exemplo prático: Worker distribuído" — Worker
//!
//! Worker gRPC que se registra no coordenador, recebe seu shard (hash por
//! arquivo) e processa cada Parquet reportando progresso via streaming.

use anyhow::{Context, Result};
use clap::Parser;
use orquestracao_distribuido::orchestrator::{
    orchestrator_client::OrchestratorClient, GetWorkRequest, HeartbeatRequest, ProgressRequest,
    RegisterRequest,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::info;

#[derive(Parser, Debug)]
#[command(name = "worker")]
struct Args {
    /// ID único do worker (ex: 1, 2, 3)
    #[arg(long)]
    id: String,

    /// Endereço do coordenador
    #[arg(long, default_value = "http://127.0.0.1:50051")]
    coordinator: String,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();
    let worker_id = format!("worker-{}", args.id);
    println!("Worker {worker_id} conectando em {}", args.coordinator);

    let mut client = OrchestratorClient::connect(args.coordinator.clone())
        .await
        .context("conectando ao coordenador")?;

    // Registro
    let resp = client
        .register_worker(RegisterRequest {
            worker_id: worker_id.clone(),
            addr: format!("worker-{}", args.id),
        })
        .await
        .context("register")?;
    println!("Registro: {:?}", resp.into_inner());

    // Heartbeat em background a cada 2s
    let mut hb_client = client.clone();
    let hb_id = worker_id.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            interval.tick().await;
            let _ = hb_client
                .heartbeat(HeartbeatRequest {
                    worker_id: hb_id.clone(),
                    timestamp_ms: now_ms(),
                })
                .await;
        }
    });

    // Pede trabalho
    let work = client
        .get_work(GetWorkRequest {
            worker_id: worker_id.clone(),
        })
        .await
        .context("get_work")?
        .into_inner();
    println!(
        "Shard {}/{} com {} arquivos: {:?}",
        work.shard_id,
        work.total_shards,
        work.files.len(),
        work.files
    );

    if work.files.is_empty() {
        println!("Nenhum arquivo para este worker — fim");
        return Ok(());
    }

    // Streaming de progresso: canal limitado para backpressure
    let (tx, rx) = mpsc::channel(10);
    let stream = ReceiverStream::new(rx);

    // Tarefa que processa arquivos e envia progresso
    let progress_tx = tx.clone();
    let files = work.files.clone();
    let total = files.len() as i32;
    let wid = worker_id.clone();
    tokio::spawn(async move {
        for (i, file) in files.iter().enumerate() {
            // Simula processamento (ex: leitura Parquet + transformação Polars)
            info!("worker {wid} processando {file} ({}/{total})", i + 1);
            tokio::time::sleep(Duration::from_millis(500)).await;

            // Simula falha ocasional para demo (worker 2 falha no arquivo 2)
            let status = if wid == "worker-2" && i == 1 {
                "error"
            } else {
                "running"
            };

            let _ = progress_tx
                .send(ProgressRequest {
                    worker_id: wid.clone(),
                    file: file.clone(),
                    processed: (i + 1) as i32,
                    total,
                    status: status.to_string(),
                })
                .await;

            if status == "error" {
                eprintln!("worker {wid} simulou erro em {file} — continua");
            }
        }
        // Último progresso com status done
        let _ = progress_tx
            .send(ProgressRequest {
                worker_id: wid.clone(),
                file: "".to_string(),
                processed: total,
                total,
                status: "done".to_string(),
            })
            .await;
    });
    drop(tx); // fecha o sender original, mantém só o da task

    // Envia stream ao coordenador e aguarda ack
    let resp = client
        .report_progress(stream)
        .await
        .context("report_progress")?;
    println!("Progresso ack: {:?}", resp.into_inner());

    // Mantém worker vivo para heartbeat
    println!("Worker {worker_id} concluído — mantendo heartbeat por 5s");
    tokio::time::sleep(Duration::from_secs(5)).await;

    Ok(())
}
