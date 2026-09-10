# 10 — Observabilidade e Produção

**Nível:** Avançado
**Estado do ecossistema:** 🟢 tracing · 🟢 criterion

## Conteúdo

1. **tracing** — instrumentação estruturada (spans, eventos), integração com OpenTelemetry para exportar traces/métricas para backends como Jaeger ou Grafana Tempo
2. **criterion** — benchmarking estatisticamente rigoroso, detecção de regressões de performance entre commits
3. **Testes de propriedade com proptest** — gerar casos de teste automaticamente a partir de invariantes (útil para validar parsers e transformações de dados contra entradas adversariais)
4. **Deploy: Docker multi-stage build** — imagens mínimas para binários Rust (build em uma stage, runtime em `distroless` ou `scratch`), reduzindo superfície de ataque e tamanho de imagem

## Exemplos práticos

| Artefato | Arquivo | O que demonstra |
|----------|---------|----------------|
| `instrumented_pipeline` | `src/bin/instrumented_pipeline.rs` | Pipeline ETL do Módulo 2 com spans (`#[instrument]`, `info_span`) por etapa; exportador OTLP/Jaeger via feature `otel` |
| `pipeline_bench` | `benches/pipeline_bench.rs` | `criterion` (harness=false) comparando `sequencial_iter` vs `lazy_paralelo`, relatório HTML |
| `Dockerfile` | `Dockerfile` | Multi-stage (cache de deps, binário em `distroless`) |

```bash
docker compose up -d  # sobe Jaeger local (:16686 UI, :4317 OTLP)
cargo run --bin instrumented_pipeline
cargo bench
# com OTLP: cargo run --features otel --bin instrumented_pipeline
```

## Exercício

Adicione testes de propriedade (`proptest`) ao parser do Módulo 1, gerando entradas aleatórias (incluindo malformadas) e validando que o parser nunca entra em pânico, sempre retornando um `Result` bem formado. Solução em `solucao/exercicio_proptest_parser.rs` (bin `exercicio_proptest_parser`) — 4 propriedades (`nunca_panica`, `bem_formada_parseia`, `campos_errados_falham`, `id_invalido_falha`).

```bash
cargo run --bin exercicio_proptest_parser
cargo test --bin exercicio_proptest_parser
```

## Leituras complementares

- [tracing](https://docs.rs/tracing)
- [criterion.rs](https://bheisler.github.io/criterion.rs/book/)
- [proptest](https://docs.rs/proptest)
