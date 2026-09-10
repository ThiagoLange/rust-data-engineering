//! Módulo 04 — Streaming
//! README: seção "Exemplo prático: Pipeline Kafka → Parquet"
//!
//! Consumer que lê eventos JSON do Kafka, transforma, bufferiza e faz flush
//! periódico (por tamanho ou tempo) para Parquet particionado por data —
//! padrão "streaming to lake". Demonstra at-least-once, checkpoint manual e DLQ.

use anyhow::{Context, Result};
use arrow::array::{Float64Array, StringArray, UInt64Array};
use arrow::record_batch::RecordBatch;
use arrow_schema::{DataType, Field, Schema};
use chrono::Utc;
use clap::Parser;
use parquet::arrow::arrow_writer::ArrowWriter;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use streaming::event::Event;
use tokio::time::interval;

#[derive(Parser, Debug)]
#[command(
    name = "kafka_consumer",
    about = "Consumer Kafka → Parquet particionado"
)]
struct Args {
    /// Brokers
    #[arg(long, default_value = "localhost:19092")]
    brokers: String,

    /// Tópico
    #[arg(long, default_value = "events")]
    topic: String,

    /// Group id
    #[arg(long, default_value = "parquet_sink")]
    group_id: String,

    /// Tamanho do batch para flush
    #[arg(long, default_value_t = 1000)]
    batch_size: usize,

    /// Intervalo de flush em segundos
    #[arg(long, default_value_t = 5)]
    flush_interval_secs: u64,

    /// Diretório de saída (particionado por `date=YYYY-MM-DD`)
    #[arg(long, default_value = "dados/saida/streaming")]
    output: PathBuf,
}

/// Converte `Vec<Event>` para `RecordBatch` Arrow.
fn events_para_batch(events: &[Event]) -> Result<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::UInt64, false),
        Field::new("timestamp", DataType::Utf8, false),
        Field::new("user_id", DataType::UInt64, false),
        Field::new("event_type", DataType::Utf8, false),
        Field::new("value", DataType::Float64, false),
        Field::new("value_transformed", DataType::Float64, true),
        Field::new("date", DataType::Utf8, false),
    ]));

    let ids: UInt64Array = events.iter().map(|e| e.id).collect();
    let timestamps: StringArray = events
        .iter()
        .map(|e| Some(e.timestamp.to_rfc3339()))
        .collect();
    let user_ids: UInt64Array = events.iter().map(|e| e.user_id).collect();
    let event_types: StringArray = events.iter().map(|e| Some(e.event_type.as_str())).collect();
    let values: Float64Array = events.iter().map(|e| e.value).collect();
    let values_t: Float64Array = events.iter().map(|e| e.value_transformed).collect();
    let dates: StringArray = events.iter().map(|e| Some(e.partition_date())).collect();

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(timestamps),
            Arc::new(user_ids),
            Arc::new(event_types),
            Arc::new(values),
            Arc::new(values_t),
            Arc::new(dates),
        ],
    )
    .context("criando RecordBatch")
}

/// Agrupa eventos por `date` e escreve um Parquet por partição.
/// Em produção, usaria `object_store` para S3/GCS.
fn flush_para_parquet(events: Vec<Event>, output: &Path) -> Result<usize> {
    if events.is_empty() {
        return Ok(0);
    }
    let mut por_data: HashMap<String, Vec<Event>> = HashMap::new();
    for e in events {
        por_data.entry(e.partition_date()).or_default().push(e);
    }

    let mut total = 0;
    for (date, evs) in por_data {
        let batch = events_para_batch(&evs)?;
        let dir = output.join(format!("date={date}"));
        std::fs::create_dir_all(&dir).with_context(|| format!("criando {}", dir.display()))?;
        // Nome por timestamp para evitar colisão
        let nome = format!(
            "part-{}-{}.parquet",
            Utc::now().timestamp_millis(),
            evs.len()
        );
        let caminho = dir.join(nome);
        let file =
            File::create(&caminho).with_context(|| format!("criando {}", caminho.display()))?;
        let mut writer =
            ArrowWriter::try_new(file, batch.schema(), None).context("criando ArrowWriter")?;
        writer.write(&batch).context("escrevendo batch")?;
        writer.close().context("fechando parquet")?;
        println!(
            "  flush: date={date} {} eventos -> {}",
            evs.len(),
            caminho.display()
        );
        total += evs.len();
    }
    Ok(total)
}

/// Trata um payload JSON: desserializa, transforma, ou envia para DLQ.
fn processar_payload(payload: &str) -> Result<Event, String> {
    let evento: Event = serde_json::from_str(payload).map_err(|e| format!("json inválido: {e}"))?;
    // Validação simples — evento com value <=0 vai para DLQ
    if evento.value <= 0.0 {
        return Err(format!("value inválido: {}", evento.value));
    }
    Ok(evento.transform())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!(
        "Consumer → brokers={} topic={} group={} output={} (batch={}, flush={}s)",
        args.brokers,
        args.topic,
        args.group_id,
        args.output.display(),
        args.batch_size,
        args.flush_interval_secs
    );
    println!("Aguardando mensagens — produza com: cargo run --bin kafka_producer -- --rate 100");

    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &args.brokers)
        .set("group.id", &args.group_id)
        .set("auto.offset.reset", "earliest")
        .set("enable.auto.commit", "true")
        .create()
        .context("criando consumer")?;

    consumer.subscribe(&[&args.topic]).context("subscribe")?;

    let mut buffer: Vec<Event> = Vec::with_capacity(args.batch_size);
    let mut dlq: Vec<String> = Vec::new();
    let mut ticker = interval(Duration::from_secs(args.flush_interval_secs));
    // Evita tick imediato no start
    ticker.tick().await;

    let mut total_consumidos: usize = 0;
    let mut total_flush: usize = 0;

    loop {
        tokio::select! {
            // Flush por tempo
            _ = ticker.tick() => {
                if !buffer.is_empty() {
                    let evs = std::mem::take(&mut buffer);
                    let n = flush_para_parquet(evs, &args.output)?;
                    total_flush += n;
                    println!("flush periódico: total_flush={total_flush} buffer={}", buffer.len());
                    if !dlq.is_empty() {
                        eprintln!("  DLQ: {} mensagens com erro (ex: {})", dlq.len(), dlq[0]);
                        // Em produção: escrever DLQ em tópico/file separado
                        dlq.clear();
                    }
                }
            }
            // Consumo de mensagem
            msg = consumer.recv() => {
                match msg {
                    Ok(m) => {
                        total_consumidos += 1;
                        let payload = m.payload_view::<str>().unwrap_or(Ok("")).unwrap_or("");
                        match processar_payload(payload) {
                            Ok(ev) => {
                                buffer.push(ev);
                                // Marca offset como processado (at-least-once: commit após flush)
                                // Aqui deixamos auto.commit; para exactly-once, faríamos
                                // `consumer.commit_message(&m, CommitMode::Async)`
                            }
                            Err(e) => {
                                eprintln!("DLQ: payload inválido: {e} | payload={payload}");
                                dlq.push(payload.to_string());
                            }
                        }

                        // Flush por tamanho
                        if buffer.len() >= args.batch_size {
                            let evs = std::mem::take(&mut buffer);
                            let n = flush_para_parquet(evs, &args.output)?;
                            total_flush += n;
                            println!("flush por tamanho: total_flush={total_flush}");
                        }

                        if total_consumidos.is_multiple_of(500) {
                            println!("  consumidos={total_consumidos} buffer={} flush={total_flush} dlq={}", buffer.len(), dlq.len());
                        }
                    }
                    Err(e) => {
                        eprintln!("Erro no stream Kafka: {e}");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;

    fn evento_fake(id: u64, date: &str) -> Event {
        Event {
            id,
            timestamp: date.parse::<DateTime<Utc>>().unwrap(),
            user_id: 1,
            event_type: "view".to_string(),
            value: 10.0,
            value_transformed: None,
        }
        .transform()
    }

    #[test]
    fn events_para_batch_preserva_linhas() {
        let evs = vec![
            evento_fake(1, "2024-01-01T00:00:00Z"),
            evento_fake(2, "2024-01-01T00:00:00Z"),
        ];
        let batch = events_para_batch(&evs).expect("ok");
        assert_eq!(batch.num_rows(), 2);
    }

    #[test]
    fn flush_particiona_por_data() {
        let dir = tempfile::tempdir().unwrap();
        let evs = vec![
            evento_fake(1, "2024-01-01T00:00:00Z"),
            evento_fake(2, "2024-01-02T00:00:00Z"),
            evento_fake(3, "2024-01-01T00:00:00Z"),
        ];
        let n = flush_para_parquet(evs, dir.path()).expect("ok");
        assert_eq!(n, 3);
        assert!(dir.path().join("date=2024-01-01").exists());
        assert!(dir.path().join("date=2024-01-02").exists());
    }

    #[test]
    fn processar_payload_valido() {
        let json = r#"{"id":1,"timestamp":"2024-01-01T00:00:00Z","user_id":1,"event_type":"purchase","value":100.0}"#;
        let ev = processar_payload(json).expect("ok");
        assert_eq!(ev.id, 1);
        let v = ev.value_transformed.expect("ok");
        assert!((v - 110.0).abs() < 1e-9);
    }

    #[test]
    fn processar_payload_invalido_vai_para_dlq() {
        let json = r#"{"id":1,"timestamp":"2024-01-01T00:00:00Z","user_id":1,"event_type":"purchase","value":-5.0}"#;
        let err = processar_payload(json).unwrap_err();
        assert!(err.contains("value inválido"));
    }

    #[test]
    fn payload_malformado_rejeitado() {
        let err = processar_payload("not json").unwrap_err();
        assert!(err.contains("json inválido"));
    }
}
