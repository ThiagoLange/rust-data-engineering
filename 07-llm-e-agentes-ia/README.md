# 07 — LLM e Agentes de IA

**Nível:** Avançado
**Estado do ecossistema:** 🟢 clientes de API (async-openai, reqwest) · 🟡 rig · 🔴 llama-cpp-rs (bindings, instabilidade entre versões)

## Conteúdo

1. **Clientes de API**
   - `async-openai` para OpenAI-compatible APIs
   - Chamadas diretas via `reqwest` para a API da Anthropic (útil quando não há SDK oficial atualizado)
2. **Embeddings e RAG**
   - Gerar embeddings (via API ou modelo local)
   - Indexar com HNSW — conecta diretamente com a arquitetura do **AI-Lake**: mesmo padrão de índice vetorial nativo usado no footer HNSW do formato
   - Busca por similaridade + reranking simples
3. **Frameworks de agentes**
   - `rig` — o framework de agentes mais ativo no ecossistema Rust atualmente, com abstrações para providers de LLM, embeddings e vector stores
   - `swiftide` — alternativa focada em pipelines de ingestão para RAG
   - Comparação com LangChain: Rust ainda não tem paridade de features, mas ganha em performance e footprint de deploy
4. **Function calling / tool use** — como estruturar chamadas de ferramentas nativamente em Rust, com validação de schema via `serde_json::Schema` ou `schemars`
5. **Inferência local**
   - `llama-cpp-rs` — bindings para llama.cpp, rodar modelos GGUF localmente
   - `candle` com modelos GGUF — alternativa mais "pura Rust", sem depender de bindings C++

## Exemplo prático 1: Pipeline RAG completo

`src/rag_pipeline.rs`: ingestão de documentos (Markdown/PDF) → chunking → geração de embeddings → indexação HNSW (via `hnsw_rs`, mesma lib usada no AI-Lake) → busca semântica → montagem de prompt com contexto recuperado → chamada ao LLM.

## Exemplo prático 2: Agente com tool calling

`src/agent_tools.rs`: agente construído com `rig` que tem acesso a duas ferramentas (busca em um índice de dados local e uma calculadora), decide quando chamar cada uma, e encadeia os resultados numa resposta final.

```bash
cargo run --bin rag_pipeline -- --docs ./documentos
cargo run --bin agent_tools -- "Quantos registros tem a tabela X e qual a média da coluna Y?"
```

## Exercício

Estenda o pipeline RAG para usar embeddings duplos (um para busca semântica, outro otimizado para reranking) — o mesmo padrão do `LlmContextSchema` usado no AI-Lake para melhorar qualidade de contexto em LLMs. Compare a qualidade dos resultados recuperados com/sem o embedding secundário. Solução em `solucoes/07-llm-e-agentes-ia`.

## Leituras complementares

- [rig](https://github.com/0xPlaygrounds/rig)
- [hnsw_rs](https://docs.rs/hnsw_rs)
- [llama-cpp-rs](https://github.com/utilityai/llama-cpp-rs)
