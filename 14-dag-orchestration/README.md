# 14 — DAG e Pipeline com Backfill — Todos os Bins

## Binários disponíveis

| Binário | Descrição |
|---------|-----------|
| `01_dag_scheduler` | Define DAG de ETL com petgraph, executa topologicamente |
| `02_pipeline_retry` | Retry com backoff exponencial e circuit breaker |
| `exercicio_pipeline_backfill` | Pipeline com backfill de janelas temporais |

## Como rodar

```bash
cargo run --bin 01_dag_scheduler
cargo run --bin 02_pipeline_retry
cargo run --bin exercicio_pipeline_backfill -- --data-dir ./dados
cargo test
```

## Projeto

O exercício `exercicio_pipeline_backfill` demonstra reprocessamento de janelas temporais: dado um pipeline com etapas ETL, reexecuta tarefas para intervalos de tempo específicos (backfill) usando um DAG. Cada janela temporal é um sub-DAG executado independentemente, com retry e circuit breaker.

### Pipeline backfill

```
extrair(janela=2024-01) → transformar(janela=2024-01) → carregar(janela=2024-01)
extrair(janela=2024-02) → transformar(janela=2024-02) → carregar(janela=2024-02)
...
```

Cada janela é executada sequencialmente, mas as tarefas dentro de cada janela podem ser paralelizadas conforme o DAG. O circuit breaker evita sobrecarga se uma etapa falhar repetidamente.

### Testes

Os testes de `02_pipeline_retry` verificam o circuit breaker (abre/fecha/reset) e o backoff (tentativas progressivas). Os testes de `01_dag_scheduler` verificam a ordem topológica para DAGs lineares e paralelos.