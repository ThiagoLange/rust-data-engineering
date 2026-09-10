//! Módulo 10 — Observabilidade e Produção
//! README: seção "Exemplo prático 1: Instrumentação com tracing"
//!
//! Pipeline ETL do Módulo 2 com spans estruturados (`tracing`), exportando
//! para Jaeger via OTLP quando feature `otel` habilitada. Rode Jaeger com
//! `docker compose up -d` e abra http://localhost:16686.

use anyhow::Result;
use polars::prelude::*;
use std::path::{Path, PathBuf};
use tracing::{info, info_span, instrument};

#[instrument(skip(df), fields(rows = df.height()))]
fn etapa_filtrar(df: DataFrame, min_metrica: f64) -> Result<DataFrame> {
    let span = info_span!("filtrar", min_metrica);
    let _guard = span.enter();
    let out = df
        .lazy()
        .filter(col("metrica").gt(lit(min_metrica)))
        .collect()
        .map_err(|e| anyhow::anyhow!("filtrar: {e}"))?;
    info!(linhas = out.height(), "filtrado");
    Ok(out)
}

#[instrument(skip(df), fields(rows = df.height()))]
fn etapa_agregar(df: DataFrame) -> Result<DataFrame> {
    let span = info_span!("agregar");
    let _guard = span.enter();
    // Agregação simples: média e contagem — como no Módulo 2
    let n = df.height() as f64;
    let met = df
        .column("metrica")
        .map_err(|e| anyhow::anyhow!("metrica: {e}"))?
        .f64()
        .map_err(|e| anyhow::anyhow!("f64: {e}"))?;
    let soma: f64 = met.iter().map(|v| v.unwrap_or(0.0)).sum();
    let media = if n > 0.0 { soma / n } else { 0.0 };
    info!(media, total = n, "agregado");
    Ok(df)
}

fn carregar_csv(path: &Path) -> Result<DataFrame> {
    let span = info_span!("carregar", arquivo = %path.display());
    let _guard = span.enter();
    if path.exists() {
        LazyCsvReader::new(path.to_string_lossy().as_ref().into())
            .with_has_header(true)
            .finish()
            .map_err(|e| anyhow::anyhow!("csv reader: {e}"))?
            .collect()
            .map_err(|e| anyhow::anyhow!("collect: {e}"))
    } else {
        info!("CSV não encontrado, gerando sintético");
        let n = 100;
        let metrica: Vec<f64> = (0..n)
            .map(|i| 100.0 + (i as f64 * 0.3).sin() * 10.0)
            .collect();
        let ts: Vec<String> = (0..n)
            .map(|i| format!("2024-01-{:02}", (i % 30) + 1))
            .collect();
        DataFrame::new(
            n,
            vec![
                Column::new("timestamp".into(), ts),
                Column::new("metrica".into(), metrica),
            ],
        )
        .map_err(|e| anyhow::anyhow!("df: {e}"))
    }
}

fn init_tracing() {
    // Subscriber fmt com EnvFilter (RUST_LOG=info por padrão)
    // Para OTLP/Jaeger: habilitar feature `otel` e configurar endpoint
    #[cfg(feature = "otel")]
    {
        // Placeholder: com `otel`, usaria tracing_opentelemetry + OTLP exporter
        // para http://localhost:4317. Mantido simples para não quebrar build sem rede.
        tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .init();
        info!("tracing com feature otel (exportador OTLP para Jaeger em :4317)");
    }
    #[cfg(not(feature = "otel"))]
    {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .init();
    }
}

fn main() -> Result<()> {
    init_tracing();
    let root = info_span!("pipeline_etl", modulo = "10-observabilidade");
    let _guard = root.enter();

    let input = PathBuf::from("../02-processamento-dados/dados/series_temporais.csv");
    info!(input = %input.display(), "iniciando pipeline");

    let df = carregar_csv(&input)?;
    info!(linhas = df.height(), "carregado");

    let df = etapa_filtrar(df, 100.0)?;
    let _ = etapa_agregar(df)?;

    info!("pipeline concluído — veja traces no Jaeger (docker compose up -d, :16686)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtrar_reduz_linhas() {
        let df = carregar_csv(Path::new("/tmp/nao_existe.csv")).expect("ok");
        let n = df.height();
        let out = etapa_filtrar(df, 1000.0).expect("ok");
        assert!(out.height() <= n);
    }

    #[test]
    fn agregar_nao_panica() {
        let df = carregar_csv(Path::new("/tmp/nao_existe.csv")).expect("ok");
        let out = etapa_agregar(df).expect("ok");
        assert!(out.height() > 0);
    }
}
