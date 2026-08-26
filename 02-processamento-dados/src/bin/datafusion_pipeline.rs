//! Módulo 02 — Processamento de Dados
//! README: "Exemplo prático 1: ETL comparativo"
//!
//! Este exemplo implementa o mesmo pipeline ETL de `polars_pipeline.rs`, mas
//! usando DataFusion e SQL:
//!
//!   ler CSV de transações → filtrar valores > 100 → juntar com clientes
//!   → agregar total por região → escrever Parquet
//!
//! Comparar as duas abordagens ajuda a entender quando usar a API expressiva
//! do Polars e quando usar a interface SQL do DataFusion.

use datafusion::prelude::*;
use std::path::Path;
use std::time::Instant;

async fn executar_etl(
    caminho_transacoes: &Path,
    caminho_clientes: &Path,
    caminho_saida: &Path,
) -> anyhow::Result<()> {
    let inicio = Instant::now();
    let ctx = SessionContext::new();

    // Registra os CSVs como tabelas temporárias.
    ctx.register_csv(
        "transacoes",
        caminho_transacoes.to_string_lossy().as_ref(),
        CsvReadOptions::new(),
    )
    .await?;

    ctx.register_csv(
        "clientes",
        caminho_clientes.to_string_lossy().as_ref(),
        CsvReadOptions::new(),
    )
    .await?;

    // Mesma lógica do pipeline Polars, expressa em SQL, materializada em
    // Parquet via COPY TO.
    let sql = format!(
        "COPY (
            SELECT c.regiao, SUM(t.valor) AS total_vendas, COUNT(*) AS quantidade_transacoes
            FROM transacoes t
            JOIN clientes c ON t.cliente_id = c.cliente_id
            WHERE t.valor > 100
            GROUP BY c.regiao
            ORDER BY total_vendas DESC
        ) TO '{}' STORED AS PARQUET",
        caminho_saida.to_string_lossy().replace('\\', "/")
    );
    ctx.sql(&sql).await?.collect().await?;

    println!(
        "ETL DataFusion concluído em {:?}. Saída: {}",
        inicio.elapsed(),
        caminho_saida.display()
    );
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    executar_etl(
        Path::new("dados/transacoes.csv"),
        Path::new("dados/clientes.csv"),
        Path::new("dados/saida/vendas_por_regiao_datafusion.parquet"),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn etl_datafusion_gera_parquet() {
        let saida = std::env::temp_dir().join("vendas_por_regiao_datafusion_test.parquet");
        executar_etl(
            Path::new("dados/transacoes.csv"),
            Path::new("dados/clientes.csv"),
            &saida,
        )
        .await
        .expect("ok");

        assert!(saida.exists());
        std::fs::remove_file(saida).unwrap();
    }
}
