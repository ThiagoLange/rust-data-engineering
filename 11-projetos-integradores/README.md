# 11 — Projetos Integradores

**Nível:** Avançado
**Pré-requisito:** todos os módulos anteriores

Projetos completos que combinam múltiplos módulos num sistema fim-a-fim. Cada projeto tem seu próprio subdiretório com README detalhado, arquitetura e roteiro de implementação passo a passo.

## Projeto 1 — Mini Lakehouse com busca vetorial

`projeto-1-mini-lakehouse/`

Pipeline completo: ingestão de dados → armazenamento em formato Iceberg/Delta (Módulo 3) → geração de embeddings sobre os dados → indexação HNSW nativa (Módulo 7) → camada de consulta que combina SQL (via DataFusion, Módulo 2) com busca semântica.

Esse projeto é essencialmente uma versão simplificada da arquitetura do **AI-Lake**: formato de arquivo único combinando Parquet + footer HNSW, compatível com Iceberg Spec v2. Serve como ponto de entrada prático para quem quer entender as decisões arquiteturais por trás de um Lakehouse vetor-nativo.

## Projeto 2 — Motor de busca vetorial standalone

`projeto-2-motor-busca-vetorial/`

Implementação de um motor de busca vetorial simples do zero: índice HNSW, quantização (F16 e opcionalmente Product Quantization), serialização com `bincode` + carregamento via `memmap2` para não precisar carregar o índice inteiro em memória.

## Projeto 3 — Pipeline de dados com agente de IA

`projeto-3-pipeline-com-agente/`

Sistema completo: ingestão de dados → transformação (Módulo 2) → armazenamento (Módulo 3) → agente de IA (Módulo 7) que responde perguntas em linguagem natural sobre os dados, com acesso a ferramentas (consulta SQL via DataFusion, busca vetorial) → dashboard de visualização (Módulo 9) mostrando o histórico de interações e métricas do pipeline.

## Como abordar os projetos

Cada projeto tem:
- Documento de arquitetura (`ARCHITECTURE.md`) explicando as decisões de design
- Roteiro de implementação incremental (várias branches, uma por etapa)
- Testes de integração cobrindo o fluxo completo

Recomendado: implementar sozinho primeiro, comparar com a solução de referência depois — não pular direto para o código pronto.
