# 10 — Observabilidade e Produção

**Nível:** Avançado
**Estado do ecossistema:** 🟢 tracing · 🟢 criterion

## Conteúdo

1. **tracing** — instrumentação estruturada (spans, eventos), integração com OpenTelemetry para exportar traces/métricas para backends como Jaeger ou Grafana Tempo
2. **criterion** — benchmarking estatisticamente rigoroso, detecção de regressões de performance entre commits
3. **Testes de propriedade com proptest** — gerar casos de teste automaticamente a partir de invariantes (útil para validar parsers e transformações de dados contra entradas adversariais)
4. **Deploy: Docker multi-stage build** — imagens mínimas para binários Rust (build em uma stage, runtime em `distroless` ou `scratch`), reduzindo superfície de ataque e tamanho de imagem

## Exemplo prático 1: Instrumentação com tracing

`src/instrumented_pipeline.rs`: pega o pipeline ETL do Módulo 2 e adiciona spans estruturados em cada etapa, exportando para um coletor OpenTelemetry local (via `docker-compose.yml` com Jaeger).

## Exemplo prático 2: Benchmark e regressão

`benches/pipeline_bench.rs`: benchmark com `criterion` comparando duas implementações do mesmo pipeline (ex: com/sem paralelismo), gerando relatório HTML de comparação.

```bash
docker compose up -d  # sobe Jaeger local
cargo run --bin instrumented_pipeline
cargo bench
```

## Exemplo prático 3: Dockerfile multi-stage

`Dockerfile`: build multi-stage otimizado (cache de dependências, binário final em `distroless`), com imagem final na casa de poucos MB.

## Exercício

Adicione testes de propriedade (`proptest`) ao parser do Módulo 1, gerando entradas aleatórias (incluindo malformadas) e validando que o parser nunca entra em pânico, sempre retornando um `Result` bem formado. Solução em `solucoes/10-observabilidade-producao`.

## Leituras complementares

- [tracing](https://docs.rs/tracing)
- [criterion.rs](https://bheisler.github.io/criterion.rs/book/)
- [proptest](https://docs.rs/proptest)
