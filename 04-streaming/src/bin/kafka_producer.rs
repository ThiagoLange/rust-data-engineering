//! Módulo 04 — Streaming
//! README: seção "Exemplo prático: Pipeline Kafka → Parquet"
//!
//! Produtor sintético que gera eventos JSON e publica no Redpanda/Kafka.
//! Uso: `cargo run --bin kafka_producer -- --rate 100 --brokers localhost:19092`

use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use rand::Rng;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord, Producer};
use std::time::Duration;
use streaming::event::Event;

#[derive(Parser, Debug)]
#[command(
    name = "kafka_producer",
    about = "Produtor sintético para o tópico Kafka"
)]
struct Args {
    /// Taxa de produção (eventos/segundo)
    #[arg(long, default_value_t = 100)]
    rate: u64,

    /// Brokers Kafka (ex: localhost:19092 para host, redpanda:9092 dentro do compose)
    #[arg(long, default_value = "localhost:19092")]
    brokers: String,

    /// Nome do tópico
    #[arg(long, default_value = "events")]
    topic: String,

    /// Número total de eventos (0 = infinito)
    #[arg(long, default_value_t = 0)]
    count: u64,
}

fn gerar_evento(id: u64) -> Event {
    let mut rng = rand::thread_rng();
    let tipos = ["purchase", "view", "click", "add_to_cart"];
    Event {
        id,
        timestamp: Utc::now(),
        user_id: rng.gen_range(1..1000),
        event_type: tipos[rng.gen_range(0..tipos.len())].to_string(),
        value: rng.gen_range(1.0..1000.0),
        value_transformed: None,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!(
        "Produtor → brokers={} topic={} rate={} ev/s",
        args.brokers, args.topic, args.rate
    );
    println!("Pressione Ctrl+C para parar. `docker compose up -d` deve estar rodando.");

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &args.brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .context("criando FutureProducer")?;

    // Garante que o tópico existe (Redpanda cria automaticamente, mas validamos)
    let intervalo = if args.rate == 0 {
        Duration::from_millis(10)
    } else {
        Duration::from_millis(1000 / args.rate.max(1))
    };

    let mut id: u64 = 0;
    loop {
        if args.count != 0 && id >= args.count {
            println!("Produzidos {id} eventos — fim");
            break;
        }
        let evento = gerar_evento(id).transform();
        let payload = serde_json::to_string(&evento).context("serializando evento")?;
        let key = id.to_string();

        // at-least-once: aguarda ack do broker (FutureRecord)
        let record = FutureRecord::to(&args.topic).payload(&payload).key(&key);

        // Timeout de entrega: 5s
        match producer.send(record, Duration::from_secs(5)).await {
            Ok(delivery) => {
                if id.is_multiple_of(100) {
                    println!(
                        "  produzido id={id} delivery={delivery:?} value={:.2}",
                        evento.value
                    );
                }
            }
            Err((e, _)) => {
                eprintln!("Falha ao produzir id={id}: {e}");
                // Em produção, enviar para DLQ (dead-letter queue) aqui
            }
        }

        // Métrica didática: log a cada segundo
        if id.is_multiple_of(args.rate.max(1)) && id != 0 {
            println!("  ... {id} eventos produzidos");
        }

        id += 1;
        tokio::time::sleep(intervalo).await;
    }

    // Flush pendentes antes de sair
    producer.flush(Duration::from_secs(5)).context("flush")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gerar_evento_tem_campos_validos() {
        let e = gerar_evento(1);
        assert_eq!(e.id, 1);
        assert!(e.value > 0.0);
        assert!(!e.event_type.is_empty());
    }

    #[test]
    fn evento_serializa_para_json() {
        let e = gerar_evento(42).transform();
        let s = serde_json::to_string(&e).expect("ok");
        let v: serde_json::Value = serde_json::from_str(&s).expect("ok");
        assert_eq!(v["id"], 42);
        assert!(v["value_transformed"].is_number());
    }

    #[test]
    fn taxa_calcula_intervalo() {
        let args = Args::parse_from(["prog", "--rate", "10"]);
        assert_eq!(args.rate, 10);
    }
}
