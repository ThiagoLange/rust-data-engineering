# 15 — Avaliação de LLM e RAG

**Nível:** Avançado
**Pré-requisito:** módulo 07

## Por que este módulo?

RAG sem avaliação é chute: recall alto não garante resposta fiel ao contexto. Este módulo implementa as métricas que importam em produção — **faithfulness** (resposta sustentada pelos trechos), **groundedness** (cobertura das afirmações), **recall@k** — além de observabilidade de **custo e latência por chamada**, que decide se o pipeline é viável.

## Estado do ecossistema

🟡 **Em maturação**: não há crate Rust consolidado equivalente a Ragas/DeepEval; implementamos as métricas com overlap de tokens (didático, sem custo de API) e deixamos o plug-in de juiz-LLM documentado.

## Exemplos

| Binário | O que demonstra |
|---------|-----------------|
| `01_rag_eval` | recall@k, faithfulness e groundedness sobre um golden set sintético |
| `02_llm_observability` | wrapper que registra latência, tokens e custo (USD/1k) por chamada |

```bash
cargo run --bin 01_rag_eval
cargo run --bin 02_llm_observability
cargo run --bin exercicio_rag_eval_suite
cargo test
```

## O que implementar

1. **`src/bin/01_rag_eval.rs`** — golden set (pergunta, docs relevantes, resposta esperada) + métricas por overlap de tokens normalizados
2. **`src/bin/02_llm_observability.rs`** — `LlmCall` com `latencia_ms`, `tokens_in/out`, `custo_usd`; agregador com p50/p99 e orçamento (fail se estourar)
3. **`solucao/exercicio_rag_eval_suite.rs`** — suite que roda as métricas em N casos, gera relatório JSON e falha o CI se faithfulness < limiar

## Convenções

- Métricas puras (sem I/O) para facilitar `#[cfg(test)]`
- `thiserror` para erros de avaliação; `anyhow` nos bins
- Sem `unwrap()`/`expect()` fora de testes