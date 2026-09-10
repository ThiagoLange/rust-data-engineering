//! Módulo 09 — Visualização de Dados
//! README: seção "Exemplo prático 1: Relatório automatizado"
//!
//! Consome saída do pipeline do Módulo 2 (séries temporais) e gera gráficos
//! PNG com `plotters` integrado a `Polars`. Também exporta JSON para D3/Grafana.

use anyhow::Result;
use chrono::NaiveDate;
use clap::Parser;
use plotters::prelude::*;
use polars::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "report_generator")]
struct Args {
    /// Arquivo de entrada (Parquet/CSV do Módulo 2)
    #[arg(
        long,
        default_value = "../02-processamento-dados/dados/series_temporais.csv"
    )]
    input: PathBuf,

    /// Diretório de saída para PNGs
    #[arg(long, default_value = "dados/saida/relatorio")]
    output: PathBuf,
}

fn carregar_serie(path: &Path) -> Result<DataFrame> {
    if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
        LazyFrame::scan_parquet(
            path.to_string_lossy().as_ref().into(),
            ScanArgsParquet::default(),
        )
        .map_err(|e| anyhow::anyhow!("scan parquet: {e}"))?
        .collect()
        .map_err(|e| anyhow::anyhow!("collect: {e}"))
    } else if path.exists() {
        LazyCsvReader::new(path.to_string_lossy().as_ref().into())
            .with_has_header(true)
            .finish()
            .map_err(|e| anyhow::anyhow!("csv reader: {e}"))?
            .collect()
            .map_err(|e| anyhow::anyhow!("collect: {e}"))
    } else {
        // Gera sintético se arquivo não existir
        println!(
            "Arquivo {} não encontrado — gerando sintético",
            path.display()
        );
        let mut dates = Vec::new();
        let mut values = Vec::new();
        let base = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        for i in 0..30 {
            dates.push(base + chrono::Days::new(i));
            values.push(100.0 + (i as f64 * 0.5).sin() * 20.0 + (i as f64 * 0.1));
        }
        DataFrame::new(
            dates.len(),
            vec![
                Column::new("timestamp".into(), dates),
                Column::new("metrica".into(), values),
            ],
        )
        .map_err(|e| anyhow::anyhow!("df sintético: {e}"))
    }
}

fn gerar_serie_temporal(df: &DataFrame, out: &Path) -> Result<()> {
    // Espera colunas timestamp (Date) e metrica (f64)
    let ts = df
        .column("timestamp")
        .map_err(|e| anyhow::anyhow!("timestamp: {e}"))?;
    let metrica = df
        .column("metrica")
        .map_err(|e| anyhow::anyhow!("metrica: {e}"))?
        .f64()
        .map_err(|e| anyhow::anyhow!("f64: {e}"))?;

    // Converte timestamp para NaiveDate se for Date, senão tenta String
    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let dates: Vec<NaiveDate> = if ts.dtype() == &DataType::Date {
        let ca = ts.date().map_err(|e| anyhow::anyhow!("date: {e}"))?;
        ca.phys
            .iter()
            .map(|opt| {
                opt.map(|d| epoch + chrono::Days::new(d as u64))
                    .unwrap_or(epoch)
            })
            .collect()
    } else {
        // Tenta parse String
        let s = ts.str().map_err(|e| anyhow::anyhow!("str: {e}"))?;
        s.iter()
            .map(|opt| {
                opt.and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok())
                    .unwrap_or(epoch)
            })
            .collect()
    };
    let valores: Vec<f64> = metrica.iter().map(|v| v.unwrap_or(0.0)).collect();

    let min_y = valores.iter().cloned().fold(f64::INFINITY, f64::min) - 5.0;
    let max_y = valores.iter().cloned().fold(f64::NEG_INFINITY, f64::max) + 5.0;
    let min_x = 0;
    let max_x = dates.len() as i32 - 1;

    let root = BitMapBackend::new(out, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Série Temporal — Metrica", ("sans-serif", 30))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(min_x..max_x, min_y..max_y)?;

    chart
        .configure_mesh()
        .x_desc("dias")
        .y_desc("metrica")
        .x_label_formatter(&|x| dates[*x as usize].format("%m-%d").to_string())
        .draw()?;

    chart.draw_series(LineSeries::new(
        valores.iter().enumerate().map(|(i, v)| (i as i32, *v)),
        &BLUE,
    ))?;

    // Área de distribuição (histograma simples como segunda série)
    chart.draw_series(
        valores
            .iter()
            .enumerate()
            .map(|(i, v)| Rectangle::new([(i as i32, min_y), (i as i32 + 1, *v)], BLUE.mix(0.2))),
    )?;

    root.present()?;
    println!("Gráfico série temporal salvo em {}", out.display());
    Ok(())
}

fn gerar_histograma(df: &DataFrame, out: &Path) -> Result<()> {
    let metrica = df
        .column("metrica")
        .map_err(|e| anyhow::anyhow!("metrica: {e}"))?
        .f64()
        .map_err(|e| anyhow::anyhow!("f64: {e}"))?;
    let valores: Vec<f64> = metrica.iter().map(|v| v.unwrap_or(0.0)).collect();

    // Bins simples: 10 bins
    let min = valores.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = valores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let bin_width = (max - min) / 10.0;
    let mut bins = [0; 10];
    for v in &valores {
        let mut idx = ((v - min) / bin_width).floor() as usize;
        if idx >= 10 {
            idx = 9;
        }
        bins[idx] += 1;
    }
    let max_count = *bins.iter().max().unwrap_or(&0);

    let root = BitMapBackend::new(out, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;
    let mut chart = ChartBuilder::on(&root)
        .caption("Distribuição — Histograma", ("sans-serif", 30))
        .margin(20)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(0..10, 0..max_count + 2)?;

    chart
        .configure_mesh()
        .x_desc("bins")
        .y_desc("frequência")
        .draw()?;

    chart.draw_series(
        Histogram::vertical(&chart)
            .style(BLUE.mix(0.7).filled())
            .margin(2)
            .data(bins.iter().enumerate().map(|(i, c)| (i as i32, *c))),
    )?;

    root.present()?;
    println!("Histograma salvo em {}", out.display());
    Ok(())
}

fn exportar_json(df: &DataFrame, out: &Path) -> Result<()> {
    // Exporta para D3/Grafana — JSON com dados agregados
    let json = serde_json::json!({
        "rows": df.height(),
        "columns": df.get_column_names().iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        "sample": df.head(Some(3)).to_string(),
    });
    std::fs::create_dir_all(out.parent().unwrap())?;
    std::fs::write(out, serde_json::to_string_pretty(&json)?)?;
    println!("JSON para D3/Grafana salvo em {}", out.display());
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("=== Relatório Automatizado — Módulo 09 ===\n");
    println!("Input: {}", args.input.display());
    println!("Output: {}", args.output.display());

    let df = carregar_serie(&args.input)?;
    println!(
        "\nDataFrame: {} linhas, colunas: {:?}",
        df.height(),
        df.get_column_names()
    );
    println!("{}", df.head(Some(5)));

    std::fs::create_dir_all(&args.output)?;

    gerar_serie_temporal(&df, &args.output.join("serie_temporal.png"))?;
    gerar_histograma(&df, &args.output.join("histograma.png"))?;
    exportar_json(&df, &args.output.join("dados_d3.json"))?;

    println!("\n✓ Relatório gerado em {}", args.output.display());
    println!(
        "  Use em pipeline batch: cargo run --bin 01_report_generator -- --input dados.parquet"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn gerar_serie_e_histograma() {
        let dir = TempDir::new().unwrap();
        let df = carregar_serie(Path::new(
            "../02-processamento-dados/dados/series_temporais.csv",
        ))
        .expect("ok");
        gerar_serie_temporal(&df, &dir.path().join("serie.png")).expect("ok");
        gerar_histograma(&df, &dir.path().join("hist.png")).expect("ok");
        assert!(dir.path().join("serie.png").exists());
        assert!(dir.path().join("hist.png").exists());
    }

    #[test]
    fn exportar_json_ok() {
        let dir = TempDir::new().unwrap();
        let df = carregar_serie(Path::new(
            "../02-processamento-dados/dados/series_temporais.csv",
        ))
        .expect("ok");
        let out = dir.path().join("out.json");
        exportar_json(&df, &out).expect("ok");
        assert!(out.exists());
    }
}
