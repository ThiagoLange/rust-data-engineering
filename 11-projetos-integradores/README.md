# 11 — Projetos Integradores

**Nível:** Avançado
**Pré-requisito:** todos os módulos anteriores

Projetos completos que combinam múltiplos módulos num sistema fim-a-fim. Cada projeto tem seu próprio subdiretório com README detalhado, arquitetura e roteiro de implementação passo a passo.

## Projetos (bins + docs)

| Binário | Arquivo | Docs | O que demonstra |
|---------|---------|------|----------------|
| `projeto_1_mini_lakehouse` | `src/bin/projeto_1_mini_lakehouse.rs` | `projeto-1-mini-lakehouse/` (`README.md`, `ARCHITECTURE.md`) | Ingestão → Parquet particionado `categoria=` com footer HNSW (M03) → `fake_embed` (M07) → `Hnsw<DistL2>` → consulta híbrida SQL Polars + vetorial |
| `projeto_2_motor_busca` | `src/bin/projeto_2_motor_busca.rs` | `projeto-2-motor-busca-vetorial/` | HNSW + quantização F16 (`half`, 2x menor) + `bincode` + `memmap2`, top-1 preservado |
| `projeto_3_pipeline_agente` | `src/bin/projeto_3_pipeline_agente.rs` | `projeto-3-pipeline-com-agente/` | Ingestão → `group_by` Polars (M02) → Parquet (M03) → agente 2 tools SQL+vetorial (M07) → `metricas.json` dashboard (M09) |

```bash
cargo run --bin projeto_1_mini_lakehouse
cargo run --bin projeto_2_motor_busca
cargo run --bin projeto_3_pipeline_agente -- --pergunta "Qual o total de vendas no Sudeste?"
cargo test
```

## Projeto 1 — Mini Lakehouse com busca vetorial

Pipeline completo: ingestão de dados → armazenamento em formato Parquet com footer HNSW estilo Iceberg/Delta (Módulo 3) → geração de embeddings sobre os dados → indexação HNSW nativa (Módulo 7) → camada de consulta que combina SQL (via Polars, Módulo 2) com busca semântica.

Esse projeto é essencialmente uma versão simplificada da arquitetura do **AI-Lake**: formato de arquivo único combinando Parquet + footer HNSW, compatível com Iceberg Spec v2. Serve como ponto de entrada prático para quem quer entender as decisões arquiteturais por trás de um Lakehouse vetor-nativo.

## Projeto 2 — Motor de busca vetorial standalone

Implementação de um motor de busca vetorial simples do zero: índice HNSW, quantização F16, serialização com `bincode` + carregamento via `memmap2` para não precisar carregar o índice inteiro em memória.

## Projeto 3 — Pipeline de dados com agente de IA

Sistema completo: ingestão de dados → transformação (Módulo 2) → armazenamento (Módulo 3) → agente de IA (Módulo 7) que responde perguntas em linguagem natural sobre os dados, com acesso a ferramentas (consulta SQL via Polars, busca vetorial) → dashboard de visualização (Módulo 9) mostrando o histórico de interações e métricas do pipeline.

## Como abordar os projetos

Cada projeto tem:
- Documento de arquitetura (`ARCHITECTURE.md`) explicando as decisões de design
- Roteiro de implementação incremental (várias branches, uma por etapa)
- Testes de integração cobrindo o fluxo completo

Recomendado: implementar sozinho primeiro, comparar com a solução de referência depois — não pular direto para o código pronto.
