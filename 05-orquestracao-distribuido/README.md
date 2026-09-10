# 05 — Orquestração e Sistemas Distribuídos

**Nível:** Avançado
**Estado do ecossistema:** 🟢 tonic (gRPC)

## Conteúdo

1. **tonic** — implementação gRPC para Rust, com Protocol Buffers via `prost`
2. **Padrões de comunicação entre serviços de dados** — request/response vs streaming bidirecional (útil para transferir grandes volumes de dados entre workers)
3. **Sharding e particionamento** — estratégias (hash, range, consistent hashing) e como implementá-las para distribuir trabalho entre workers
4. **Coordenação distribuída** — abordagens simples (leader election básico) sem entrar em consensus completo (Raft fica fora do escopo, mas é citado como próximo passo)

## Exemplo prático: Worker distribuído

- `src/bin/coordinator.rs`: coordenador gRPC (`OrchestratorServer` em `127.0.0.1:50051`) que mantém `files: Vec<String>` (12 Parquet sintéticos), distribui via sharding por hash (`shard_for_file` em `src/lib.rs:14`) e recebe progresso via `ReportProgress` (client streaming).
- `src/bin/worker.rs`: worker que se registra (`RegisterWorker`), mantém heartbeat a cada 2s (`Heartbeat`), pede shard (`GetWork`) e processa arquivos reportando `ProgressRequest {status}` via streaming. Tipos em `src/lib.rs` e proto em `proto/orchestrator.proto`.
- `src/lib.rs:26` — `shard_files`, `range_shard` (hash vs range, consistent hashing).
- `build.rs:1` — `tonic-build` compila o proto.

```bash
# terminal 1
cargo run --bin coordinator
# terminais 2-4
cargo run --bin worker -- --id 1 --coordinator http://127.0.0.1:50051
cargo run --bin worker -- --id 2 --coordinator http://127.0.0.1:50051
cargo run --bin worker -- --id 3 --coordinator http://127.0.0.1:50051
```

## Exercício

Adicione tolerância a falhas simples: se um worker cair no meio do processamento, o coordenador deve detectar (timeout/heartbeat 5s, check a cada 2s) e reatribuir o shard a outro worker vivo. Solução em `solucao/exercicio_tolerancia_falhas.rs` (bin `exercicio_tolerancia_falhas`) — `CoordinatorState::check_heartbeats` + `shard_owner` reatribuição, 3 testes.

```bash
cargo run --bin exercicio_tolerancia_falhas
# mate um worker e veja o log: "shard X reatribuído de worker-Y para worker-Z"
```

## Leituras complementares

- [tonic docs](https://docs.rs/tonic)
- [gRPC concepts](https://grpc.io/docs/what-is-grpc/core-concepts/)
