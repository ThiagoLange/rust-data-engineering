//! Módulo 02 — Processamento de Dados
//! README: "Exemplo prático 1: ETL comparativo"
//!
//! Este exemplo implementa um pipeline ETL simples com Polars:
//!
//!   ler CSV de transações → filtrar valores > 100 → juntar com clientes
//!   → agregar total por região → escrever Parquet
//!
//! O objetivo é mostrar a API lazy do Polars para pipelines reais e comparar
//! posteriormente com a mesma lógica em DataFusion.

use polars::prelude::*;
use std::path::Path;
use std::time::Instant;

fn executar_etl(
    caminho_transacoes: &Path,
    caminho_clientes: &Path,
    caminho_saida: &Path,
) -> anyhow::Result<()> {
    let inicio = Instant::now();

    let transacoes = LazyCsvReader::new(caminho_transacoes.to_string_lossy().as_ref().into())
        .with_has_header(true)
        .finish()?;

    let clientes = LazyCsvReader::new(caminho_clientes.to_string_lossy().as_ref().into())
        .with_has_header(true)
        .finish()?;

    let resultado = transacoes
        .filter(col("valor").gt(lit(100.0)))
        .join(
            clientes,
            [col("cliente_id")],
            [col("cliente_id")],
            JoinType::Inner.into(),
        )
        .group_by([col("regiao")])
        .agg([
            col("valor").sum().alias("total_vendas"),
            col("id").count().alias("quantidade_transacoes"),
        ])
        .sort(
            ["total_vendas"],
            SortMultipleOptions::default().with_order_descending(true),
        );

    // Escreve o resultado em Parquet. O formato colunar é muito mais eficiente
    // para leituras analíticas subsequentes do que CSV.
    let mut df = resultado.collect()?;
    let mut arquivo = std::fs::File::create(caminho_saida)?;
    ParquetWriter::new(&mut arquivo).finish(&mut df)?;

    println!(
        "ETL Polars concluído em {:?}. Saída: {}",
        inicio.elapsed(),
        caminho_saida.display()
    );
    Ok(())
}

fn main() -> anyhow::Result<()> {
    executar_etl(
        Path::new("dados/transacoes.csv"),
        Path::new("dados/clientes.csv"),
        Path::new("dados/saida/vendas_por_regiao_polars.parquet"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etl_polars_gera_parquet() {
        let saida = std::env::temp_dir().join("vendas_por_regiao_polars_test.parquet");
        executar_etl(
            Path::new("dados/transacoes.csv"),
            Path::new("dados/clientes.csv"),
            &saida,
        )
        .expect("ok");

        assert!(saida.exists());
        std::fs::remove_file(saida).unwrap();
    }
}
