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

## Exemplos práticos

| Binário | Arquivo | O que demonstra |
|---------|---------|----------------|
| `01_report_generator` | `src/bin/01_report_generator.rs` | Relatório batch: consome CSV/Parquet do Módulo 2, gera `serie_temporal.png` + `histograma.png` via `plotters`, exporta `dados_d3.json` para D3/Grafana |
| `02_dashboard` | `src/bin/02_dashboard.rs` | Dashboard `egui`/`eframe`: sliders, ComboBox de categoria, tabela filtrada, barras de progresso |

```bash
cargo run --bin 01_report_generator -- --input ../02-processamento-dados/dados/series_temporais.csv
cargo run --bin 02_dashboard ../02-processamento-dados/dados/series_temporais.csv
```

## Exercício

Estenda o dashboard `egui` para consumir dados em tempo real do pipeline de streaming do Módulo 4 (via um channel Tokio), atualizando os gráficos conforme novos eventos chegam. Solução em `solucao/exercicio_dashboard_streaming.rs` (bin `exercicio_dashboard_streaming`) — produtor `mpsc` a cada 200ms + `try_recv` com `request_repaint`.

```bash
cargo run --bin exercicio_dashboard_streaming
```

## Leituras complementares

- [plotters](https://docs.rs/plotters)
- [egui](https://www.egui.rs/)
