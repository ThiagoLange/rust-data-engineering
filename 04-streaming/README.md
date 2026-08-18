# 04 — Streaming

**Nível:** Intermediário
**Estado do ecossistema:** 🟢 rdkafka · 🟢 tokio-stream

## Conteúdo

1. **rdkafka** — bindings Rust sobre librdkafka (C), produtor e consumidor Kafka com garantias de entrega configuráveis
2. **tokio-stream** — abstrações de stream assíncrono, combinadores (`map`, `filter`, `throttle`)
3. **Backpressure** — por que importa em streaming e como implementar com channels limitados (`tokio::sync::mpsc` com capacidade)
4. **Padrões de processamento**: at-least-once vs exactly-once, checkpointing manual, dead-letter queues

## Exemplo prático: Pipeline Kafka → Parquet

`src/kafka_consumer.rs`: consumer que lê eventos JSON de um tópico Kafka, aplica uma transformação, faz buffer em memória e faz flush periódico (por tempo ou tamanho) para arquivos Parquet particionados — um padrão comum de "streaming to lake".

Inclui `docker-compose.yml` com Kafka local (via Redpanda, mais leve que Kafka completo) para rodar o exemplo sem infraestrutura externa.

```bash
docker compose up -d
cargo run --bin kafka_consumer
# em outro terminal, produzir eventos de teste:
cargo run --bin kafka_producer -- --rate 100
```

## Exercício

Adicione backpressure ao consumer: se o sink (escrita em Parquet) ficar mais lento que a ingestão do Kafka, o consumer deve pausar o consumo em vez de acumular memória indefinidamente. Meça o comportamento sob carga com o produtor de teste. Solução em `solucoes/04-streaming`.

## Leituras complementares

- [rdkafka docs](https://docs.rs/rdkafka)
- [Tokio streams](https://tokio.rs/tokio/tutorial/streams)
