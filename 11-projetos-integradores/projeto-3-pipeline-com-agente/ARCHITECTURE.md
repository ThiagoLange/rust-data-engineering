# Projeto 3 — Pipeline de dados com agente de IA — Arquitetura

Sistema fim-a-fim: ingestão → transformação → Parquet → agente NL → dashboard JSON.

## Etapas

1. **Ingestão** (Módulo 1/2): CSV sintético de vendas → Polars.
2. **Transformação** (Módulo 2): lazy `filter` + `group_by` por região (total, média).
3. **Armazenamento** (Módulo 3): Parquet particionado por `regiao=`.
4. **Agente** (Módulo 7): 2 tools — `sql_consulta` (agregados sobre o DF em memória)
   e `busca_vetorial` (HNSW sobre descrições); decisão por heurística + resposta NL.
5. **Dashboard** (Módulo 9): `metricas.json` com histórico de interações
   (pergunta, tools usadas, latência) pronto para D3/Grafana/egui.

## Roteiro incremental

- `etapa-1`: ingestão + transformação + Parquet.
- `etapa-2`: agente com 2 tools.
- `etapa-3`: dashboard JSON + métricas.

## Testes

`cargo test --bin projeto_3_pipeline_agente`: agregação correta, tools respondem,
JSON válido.
