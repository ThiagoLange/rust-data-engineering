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

## Exemplos práticos

| Binário | Arquivo | O que demonstra |
|---------|---------|----------------|
| `01_rag_pipeline` | `src/bin/01_rag_pipeline.rs` | Ingestão Markdown → `chunk_text` (500/50) → embeddings (`async-openai` se `OPENAI_API_KEY` ou `fake_embed` sintético) → HNSW `hnsw_rs` (`Hnsw<DistDot>`, mesmo footer AI-Lake) → busca `top-k` + reranking lexical → `call_llm` (`async-openai`/`reqwest` Anthropic ou simulado) — custo limitado a 512 tokens |
| `02_agent_tools` | `src/bin/02_agent_tools.rs` | Agente com 2 ferramentas (`buscar_dados` + `calculadora`), schemas via `schemars`, decisão por heurística, encadeamento, `schemars::schema_for` — alternativa leve a `rig` |

```bash
cargo run --bin 01_rag_pipeline -- --docs ./documentos --query "O que é Rust?"
cargo run --bin 02_agent_tools -- "Quantos registros tem a tabela vendas e qual a média de preco_unitario?"
# com API real (ver .env.example):
# OPENAI_API_KEY=sk-... cargo run --bin 01_rag_pipeline
```

## Exercício

Estenda o pipeline RAG para usar embeddings duplos (um para busca semântica, outro otimizado para reranking) — mesmo padrão do `LlmContextSchema` do AI-Lake. Solução em `solucao/exercicio_double_embeddings.rs` (bin `exercicio_double_embeddings`) — `fake_embed` (busca) + `fake_embed_rerank` (reranking) com 2 HNSW (`DistL2`), `recall@k` com/sem segundo embedding, footer dual no AI-Lake.

```bash
cargo run --bin exercicio_double_embeddings
```

## Leituras complementares

- [rig](https://github.com/0xPlaygrounds/rig)
- [hnsw_rs](https://docs.rs/hnsw_rs)
- [llama-cpp-rs](https://github.com/utilityai/llama-cpp-rs)
