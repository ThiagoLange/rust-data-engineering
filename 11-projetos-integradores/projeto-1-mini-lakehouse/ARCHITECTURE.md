# Projeto 1 — Mini Lakehouse com busca vetorial — Arquitetura

Versão simplificada do **AI-Lake**: Parquet colunar + footer HNSW + camada SQL + busca semântica.

## Etapas

1. **Ingestão** (Módulo 1/2): CSV sintético de produtos → `DataFrame` Polars.
2. **Armazenamento** (Módulo 3): Parquet com `key_value_metadata` simulando footer HNSW
   (`ailake.index_type=hnsw`), particionado por `categoria=`.
3. **Embeddings** (Módulo 7): `fake_embed` determinístico (32d, L2-norm) sobre
   `nome + descricao`. Em produção: `async-openai` ou modelo local.
4. **Índice HNSW** (Módulo 7): `hnsw_rs::Hnsw<DistL2>`, mesmo padrão do footer AI-Lake.
5. **Consulta híbrida** (Módulo 2): filtro SQL-like via Polars lazy
   (`categoria == X AND preco < Y`) + busca vetorial top-k + fusão por score.

## Roteiro incremental

- `etapa-1`: ingestão + Parquet particionado.
- `etapa-2`: embeddings + HNSW.
- `etapa-3`: consulta híbrida SQL + vetorial.

## Testes de integração

`cargo test --bin projeto_1_mini_lakehouse`: round-trip Parquet, recall do HNSW,
fusão híbrida retorna relevante.
