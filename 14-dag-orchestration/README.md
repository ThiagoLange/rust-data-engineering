# 14 — Orquestração de Pipelines (DAGs)

**Nível:** Avançado
**Pré-requisito:** módulos 04–05

## Por que este módulo?

Pipelines de dados precisam de orquestração: definir dependências entre etapas (DAGs), gerenciar retries, backfills, paralelismo e monitoramento. Este módulo mostra como construir orquestradores de pipelines em Rust, no espírito de ferramentas como Airflow e Dagster.

## Estado do ecossistema

🟡 **Em maturação**: `petgraph` para DAGs + `tokio` para scheduler. Não há orquestrador Rust maduro equivalente ao Airflow; usamos implementação própria sobre `petgraph`.

## Exemplos

| Binário | O que demonstra |
|---------|-----------------|
| `01_dag_scheduler` | DAG de ETL com `petgraph`, execução em ordem topológica |
| `02_pipeline_retry` | Retry com backoff exponencial e circuit breaker por tarefa |
| `exercicio_pipeline_backfill` | Backfill: reprocessamento de janelas temporais como sub-DAGs |

```bash
cargo run --bin 01_dag_scheduler
cargo run --bin 02_pipeline_retry
cargo run --bin exercicio_pipeline_backfill
cargo test
```

## O que implementar

1. **`src/bin/01_dag_scheduler.rs`** — Define um DAG de processamento (ETL), executa topologicamente via `petgraph`
2. **`src/bin/02_pipeline_retry.rs`** — Cada tarefa com retry/backoff e circuit breaker (abre após N falhas, reseta no sucesso)
3. **`solucao/exercicio_pipeline_backfill.rs`** — Pipeline com backfill (reprocessamento de janelas temporais)

## Projeto: pipeline com backfill

O exercício demonstra reprocessamento de janelas temporais: cada janela é um sub-DAG executado independentemente, com retry e circuit breaker.

```
extrair(janela=2024-01) → transformar(janela=2024-01) → carregar(janela=2024-01)
extrair(janela=2024-02) → transformar(janela=2024-02) → carregar(janela=2024-02)
...
```

Cada janela é executada sequencialmente, mas as tarefas dentro de cada janela seguem o DAG. O circuit breaker evita sobrecarga se uma etapa falhar repetidamente.

## Convenções

- Usar `petgraph` para modelar DAGs (`toposort` para ordem de execução)
- `thiserror` para erros de orquestração; `anyhow` nos bins
- Sem `unwrap()`/`expect()` fora de testes
