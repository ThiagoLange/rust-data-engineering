# 09 — Visualização de Dados

**Nível:** Intermediário
**Estado do ecossistema:** 🟢 plotters · 🟢 egui

## Conteúdo

1. **plotters** — geração de gráficos estáticos (PNG/SVG): linha, barra, dispersão, histograma. Integra bem com Polars para plotar direto de um DataFrame
2. **egui** — biblioteca de UI imediata (immediate mode), ótima para dashboards interativos locais sem depender de navegador/JS
3. **Exportação para ferramentas externas** — gerar dados em formato consumível por Observable, D3.js ou Grafana quando a visualização precisa viver fora do binário Rust (ex: dashboards compartilhados via web)

## Quando usar cada abordagem

- **plotters**: relatórios automatizados, gráficos gerados em pipelines batch (ex: relatório diário em PDF/PNG)
- **egui**: ferramentas internas interativas, dashboards de monitoramento local, prototipagem rápida de UI sobre dados
- **Exportar para D3/Grafana**: quando o consumidor final é um time que já usa essas ferramentas, ou quando a visualização precisa ser acessível via navegador para múltiplos usuários

## Exemplo prático 1: Relatório automatizado

`src/report_generator.rs`: consome a saída do pipeline do Módulo 2 (dados agregados) e gera automaticamente um conjunto de gráficos (série temporal, distribuição) como PNG, usando `plotters` integrado com Polars.

## Exemplo prático 2: Dashboard interativo

`src/dashboard.rs`: aplicação `egui` que carrega os dados processados e permite filtrar/explorar interativamente (sliders, dropdowns) com os gráficos atualizando em tempo real.

```bash
cargo run --bin report_generator -- --input dados_processados.parquet
cargo run --bin dashboard -- --input dados_processados.parquet
```

## Exercício

Estenda o dashboard `egui` para consumir dados em tempo real do pipeline de streaming do Módulo 4 (via um channel Tokio), atualizando os gráficos conforme novos eventos chegam. Solução em `solucoes/09-visualizacao-dados`.

## Leituras complementares

- [plotters](https://docs.rs/plotters)
- [egui](https://www.egui.rs/)
