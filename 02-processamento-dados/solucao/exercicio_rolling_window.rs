//! Módulo 02 — Processamento de Dados
//! README: seção "Exercício"
//!
//! Solução do exercício: agregação de janela deslizante (rolling window) sobre
//! uma série temporal usando LazyFrame do Polars.
//!
//! O objetivo é mostrar como calcular médias móveis sem perder a granularidade
//! dos dados e como inspecionar o plano de execução lazy com `.explain()`.

use polars::prelude::*;
use std::path::Path;

/// Lê a série temporal e converte a coluna `timestamp` para `Date`.
fn ler_serie(caminho: &Path) -> anyhow::Result<LazyFrame> {
    Ok(
        LazyCsvReader::new(caminho.to_string_lossy().as_ref().into())
            .with_has_header(true)
            .finish()?
            .with_columns([col("timestamp").str().to_date(StrptimeOptions {
                format: Some("%Y-%m-%d".into()),
                strict: false,
                exact: true,
                cache: true,
            })]),
    )
}

/// Calcula a média móvel de 7 dias usando `rolling`. Cada ponto mantém sua
/// data original e ganha uma coluna com a média dos 7 dias anteriores
/// (inclusive o próprio dia).
fn rolling_window_7d(lf: LazyFrame) -> anyhow::Result<DataFrame> {
    lf.rolling(
        col("timestamp"),
        [],
        RollingGroupOptions {
            index_column: "timestamp".into(),
            period: Duration::parse("7d"),
            offset: Duration::parse("0d"),
            closed_window: ClosedWindow::Left,
        },
    )
    .agg([
        col("metrica").mean().alias("media_movel_7d"),
        col("metrica").first().alias("metrica"),
    ])
    .sort(["timestamp"], SortMultipleOptions::default())
    .collect()
    .map_err(Into::into)
}

/// Mostra o plano de execução que o Polars monta para o cálculo.
fn explicar_plano(lf: LazyFrame) -> anyhow::Result<String> {
    lf.rolling(
        col("timestamp"),
        [],
        RollingGroupOptions {
            index_column: "timestamp".into(),
            period: Duration::parse("7d"),
            offset: Duration::parse("0d"),
            closed_window: ClosedWindow::Left,
        },
    )
    .agg([
        col("metrica").mean().alias("media_movel_7d"),
        col("metrica").first().alias("metrica"),
    ])
    .explain(false)
    .map_err(Into::into)
}

fn main() -> anyhow::Result<()> {
    let caminho_entrada = Path::new("dados/series_temporais.csv");
    let caminho_saida = Path::new("dados/saida/rolling_window.parquet");

    let lf = ler_serie(caminho_entrada)?;

    println!("=== Plano de execução ===");
    println!("{}", explicar_plano(lf.clone())?);

    println!("\n=== Calculando média móvel ===");
    let mut df = rolling_window_7d(lf)?;
    println!("{}", df.head(Some(20)));

    let parent = caminho_saida.parent().ok_or_else(|| {
        anyhow::anyhow!("caminho de saída sem parent: {}", caminho_saida.display())
    })?;
    std::fs::create_dir_all(parent)?;
    let mut arquivo = std::fs::File::create(caminho_saida)?;
    ParquetWriter::new(&mut arquivo).finish(&mut df)?;

    println!("\nResultado escrito em {}", caminho_saida.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serie_e_convertida_para_date() {
        let lf = ler_serie(Path::new("dados/series_temporais.csv")).expect("ok");
        let df = lf.collect().expect("ok");
        assert_eq!(df["timestamp"].dtype(), &DataType::Date);
    }

    #[test]
    fn rolling_window_preserva_numero_de_linhas() {
        let lf = ler_serie(Path::new("dados/series_temporais.csv")).expect("ok");
        let df = rolling_window_7d(lf).expect("ok");
        assert_eq!(df.height(), 500);
        assert!(df
            .get_column_names()
            .iter()
            .any(|nome| nome.as_str() == "media_movel_7d"));
    }

    #[test]
    fn plano_de_execucao_gerado() {
        let lf = ler_serie(Path::new("dados/series_temporais.csv")).expect("ok");
        let plano = explicar_plano(lf).expect("ok");
        assert!(plano.contains("AGGREGATE"));
    }
}
