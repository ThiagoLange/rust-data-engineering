# 05 — Orquestração e Sistemas Distribuídos

**Nível:** Avançado
**Estado do ecossistema:** 🟢 tonic (gRPC)

## Conteúdo

1. **tonic** — implementação gRPC para Rust, com Protocol Buffers via `prost`
2. **Padrões de comunicação entre serviços de dados** — request/response vs streaming bidirecional (útil para transferir grandes volumes de dados entre workers)
3. **Sharding e particionamento** — estratégias (hash, range, consistent hashing) e como implementá-las para distribuir trabalho entre workers
4. **Coordenação distribuída** — abordagens simples (leader election básico) sem entrar em consensus completo (Raft fica fora do escopo, mas é citado como próximo passo)

## Exemplo prático: Worker distribuído

`src/coordinator.rs` + `src/worker.rs`: um coordenador que recebe uma lista de arquivos Parquet para processar, distribui via gRPC entre N workers (sharding por hash do nome do arquivo), cada worker processa seu shard e reporta progresso via streaming gRPC de volta ao coordenador.

```bash
# terminal 1
cargo run --bin coordinator
# terminais 2-4
cargo run --bin worker -- --id 1
cargo run --bin worker -- --id 2
cargo run --bin worker -- --id 3
```

## Exercício

Adicione tolerância a falhas simples: se um worker cair no meio do processamento, o coordenador deve detectar (timeout/heartbeat) e reatribuir o shard a outro worker disponível. Solução em `solucoes/05-orquestracao-distribuido`.

## Leituras complementares

- [tonic docs](https://docs.rs/tonic)
- [gRPC concepts](https://grpc.io/docs/what-is-grpc/core-concepts/)
