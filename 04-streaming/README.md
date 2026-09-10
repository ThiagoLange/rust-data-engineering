# 04 — Streaming

**Nível:** Intermediário
**Estado do ecossistema:** 🟢 rdkafka · 🟢 tokio-stream

## Conteúdo

1. **rdkafka** — bindings Rust sobre librdkafka (C), produtor e consumidor Kafka com garantias de entrega configuráveis
2. **tokio-stream** — abstrações de stream assíncrono, combinadores (`map`, `filter`, `throttle`)
3. **Backpressure** — por que importa em streaming e como implementar com channels limitados (`tokio::sync::mpsc` com capacidade)
4. **Padrões de processamento**: at-least-once vs exactly-once, checkpointing manual, dead-letter queues

## Exemplo prático: Pipeline Kafka → Parquet

- `src/bin/kafka_producer.rs`: produtor sintético (`--rate`, `--brokers`, `--topic`) que gera `Event` JSON e publica com `FutureProducer` (at-least-once, `message.timeout.ms=5000`).
- `src/bin/kafka_consumer.rs`: consumer que lê eventos JSON, valida/transforma (`value * 1.1`), bufferiza em memória e faz flush periódico (por tamanho `batch_size` ou tempo `flush_interval_secs`) para Parquet particionado por `date=YYYY-MM-DD` — padrão "streaming to lake". Inclui DLQ em memória para payloads inválidos e checkpoint via `enable.auto.commit`.

Inclui `docker-compose.yml` com Redpanda (mais leve que Kafka completo) para rodar localmente sem infraestrutura externa.

```bash
docker compose up -d
cargo run --bin kafka_consumer -- --brokers localhost:19092 --topic events
# em outro terminal, produzir eventos de teste:
cargo run --bin kafka_producer -- --rate 100 --brokers localhost:19092
```

Tipos compartilhados em `src/event.rs` e `src/lib.rs` (`Event::transform`, `partition_date`).

## Exercício

Adicione backpressure ao consumer: se o sink (escrita em Parquet) ficar mais lento que a ingestão do Kafka, o consumer deve pausar o consumo em vez de acumular memória indefinidamente. Meça o comportamento sob carga com o produtor de teste. Solução em `solucao/exercicio_backpressure.rs` (bin `exercicio_backpressure`) — demonstra `tokio::sync::mpsc` limitado (cap=2) e `tokio-stream` (`map`/`filter`).

```bash
cargo run --bin exercicio_backpressure
```

## Leituras complementares

- [rdkafka docs](https://docs.rs/rdkafka)
- [Tokio streams](https://tokio.rs/tokio/tutorial/streams)
