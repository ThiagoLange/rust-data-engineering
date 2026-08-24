# Rust para Engenharia de Dados & IA

Treinamento prático, do básico ao avançado, para usar Rust em engenharia de dados, machine learning, LLMs/agentes de IA, visão computacional e visualização de dados.

Cada módulo é auto-contido: teoria curta + exemplos de código rodáveis + exercício com solução na pasta `solucao/` dentro do mesmo crate.

## Como usar este repositório

- Siga os módulos em ordem — cada um assume conhecimento do anterior.
- Todo módulo tem seu próprio README com contexto teórico e links para os crates usados.
- Exemplos ficam em `src/` dentro da pasta do módulo, prontos para `cargo run`.
- Exercícios têm enunciado no README e solução em `NN-nome-do-modulo/solucao/`.

## Pré-requisitos

- Rust estável mais recente (`rustup update`)
- `cargo`, `git`
- Docker (para módulos de streaming e produção)
- Familiaridade básica com engenharia de dados (SQL, formatos de arquivo, ETL) ajuda, mas não é obrigatória

## Roadmap

| # | Módulo | Nível | Foco |
|---|--------|-------|------|
| 00 | [Fundamentos](./00-fundamentos) | Básico | Ownership, traits, erros, concorrência |
| 01 | [I/O e Parsing](./01-io-e-parsing) | Básico | serde, CSV, parsing binário |
| 02 | [Processamento de Dados](./02-processamento-dados) | Intermediário | Polars, DataFusion, Arrow |
| 03 | [Armazenamento e Formatos](./03-armazenamento-formatos) | Intermediário | Parquet, Delta Lake, Iceberg |
| 04 | [Streaming](./04-streaming) | Intermediário | Kafka, processamento em tempo real |
| 05 | [Orquestração e Distribuído](./05-orquestracao-distribuido) | Avançado | gRPC, sharding, workers distribuídos |
| 06 | [Machine Learning](./06-machine-learning) | Avançado | linfa, candle, burn, ONNX |
| 07 | [LLM e Agentes de IA](./07-llm-e-agentes-ia) | Avançado | RAG, embeddings, agentes, inferência local |
| 08 | [Visão Computacional](./08-visao-computacional) | Avançado | image, opencv-rust, inferência ONNX |
| 09 | [Visualização de Dados](./09-visualizacao-dados) | Intermediário | plotters, egui, dashboards |
| 10 | [Observabilidade e Produção](./10-observabilidade-producao) | Avançado | tracing, criterion, deploy |
| 11 | [Projetos Integradores](./11-projetos-integradores) | Avançado | Pipelines fim-a-fim combinando tudo |

## Estado do ecossistema

Rust tem maturidade desigual entre as áreas cobertas aqui. Isso é sinalizado no README de cada módulo:

- 🟢 **Produção-ready**: usado em empresas reais em escala (ex: Polars, Arrow, Parquet)
- 🟡 **Em maturação**: funcional, mas com lacunas de documentação/ecossistema (ex: candle, rig)
- 🔴 **Experimental/nicho**: promissor, mas exige tolerância a instabilidade (ex: kornia-rs)

## Licença

MIT — use livremente para aprender, ensinar ou adaptar.
