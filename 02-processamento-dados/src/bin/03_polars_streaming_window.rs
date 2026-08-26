//! Módulo 02 — Processamento de Dados
//! README: seção "Conteúdo", item 1 — Streaming e window functions com Polars
//!
//! Quando o dataset não cabe na memória, o Polars pode executar operações em
//! *streaming*: ele processa o arquivo em pedaços, mantendo o uso de RAM
//! constante em vez de carregar tudo de uma vez. Ative com
//! `.with_streaming(true)` antes do `.collect()`.
//!
//! Já as *window functions* (aqui via `rolling`) permitem calcular valores em
//! uma janela ao redor de cada linha — média móvel, soma acumulada, etc. —
//! sem perder a granularidade original, como aconteceria num `group_by`.

use polars::prelude::*;
use std::path::Path;

/// Lê a série temporal como LazyFrame, convertendo a coluna de data para o
/// tipo `Date`. A conversão explícita é importante: sem ela, datas ficam como
/// strings e operações temporais não funcionam.
fn ler_serie_temporal(caminho: &Path) -> anyhow::Result<LazyFrame> {
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

/// Calcula a média móvel de 7 dias da métrica usando uma rolling window.
/// `RollingGroupOptions` define a coluna de índice (a data) e o tamanho da
/// janela (`period`). Cada ponto fica associado à média dos 7 dias anteriores.
fn media_movel_7d(lf: LazyFrame) -> anyhow::Result<DataFrame> {
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
    .agg([col("metrica").mean().alias("media_movel_7d")])
    .collect()
    .map_err(Into::into)
}

/// Executa o cálculo em modo streaming. `with_streaming(true)` diz ao Polars
/// para processar o pipeline em chunks, útil quando o dataset é maior que a
/// RAM. Para um arquivo de 500 linhas a diferença é zero — o modo didático é
/// o que importa.
fn media_movel_streaming(lf: LazyFrame) -> anyhow::Result<DataFrame> {
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
    .agg([col("metrica").mean().alias("media_movel_7d")])
    .with_streaming(true)
    .collect()
    .map_err(Into::into)
}

fn main() -> anyhow::Result<()> {
    let caminho = Path::new("dados/series_temporais.csv");
    let lf = ler_serie_temporal(caminho)?;

    println!("=== Primeiras linhas da série ===");
    println!("{}", lf.clone().collect()?.head(Some(10)));

    println!("\n=== Média móvel de 7 dias ===");
    let df = media_movel_7d(lf.clone())?;
    println!("{}", df.head(Some(15)));

    println!("\n=== Plano de execução lazy ===");
    let plano = lf
        .clone()
        .rolling(
            col("timestamp"),
            [],
            RollingGroupOptions {
                index_column: "timestamp".into(),
                period: Duration::parse("7d"),
                offset: Duration::parse("0d"),
                closed_window: ClosedWindow::Left,
            },
        )
        .agg([col("metrica").mean().alias("media_movel_7d")])
        .explain(false)?;
    println!("{plano}");

    println!("\n=== Mesmo cálculo em modo streaming ===");
    let lf = ler_serie_temporal(caminho)?;
    let df_streaming = media_movel_streaming(lf)?;
    println!("{}", df_streaming.tail(Some(5)));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serie_temporal_tem_coluna_data_convertida() {
        let lf = ler_serie_temporal(Path::new("dados/series_temporais.csv")).expect("ok");
        let df = lf.collect().expect("ok");
        assert_eq!(df["timestamp"].dtype(), &DataType::Date);
    }

    #[test]
    fn media_movel_adiciona_coluna() {
        let lf = ler_serie_temporal(Path::new("dados/series_temporais.csv")).expect("ok");
        let df = media_movel_7d(lf).expect("ok");
        assert!(df
            .get_column_names()
            .iter()
            .any(|nome| nome.as_str() == "media_movel_7d"));
        assert_eq!(df.height(), 500);
    }

    #[test]
    fn streaming_produz_mesmo_numero_de_linhas() {
        let lf = ler_serie_temporal(Path::new("dados/series_temporais.csv")).expect("ok");
        let df = media_movel_streaming(lf).expect("ok");
        assert_eq!(df.height(), 500);
    }
}
