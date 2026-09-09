//! Módulo 03 — Armazenamento e Formatos
//! README: seção "Conteúdo", item 1 — Parquet: escrita e leitura
//!
//! Parquet é o formato colunar comprimido padrão para analítica. Este exemplo
//! mostra como ler um CSV, convertê-lo para RecordBatch do Apache Arrow e
//! gravar/lê-lo em Parquet usando diretamente os crates `arrow` e `parquet`.

use anyhow::{Context, Result};
use arrow::record_batch::RecordBatch;
use arrow_csv::ReaderBuilder;
use arrow_schema::{DataType, Field, Schema};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::arrow_writer::ArrowWriter;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// Schema esperado do CSV de vendas.
fn schema_vendas() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::UInt64, false),
        Field::new("data", DataType::Utf8, false),
        Field::new("categoria", DataType::Utf8, false),
        Field::new("produto", DataType::Utf8, false),
        Field::new("quantidade", DataType::UInt32, false),
        Field::new("preco_unitario", DataType::Float64, false),
        Field::new("regiao", DataType::Utf8, false),
        Field::new("cliente_id", DataType::UInt64, false),
    ]))
}

/// Lê o CSV de vendas para uma lista de `RecordBatch`.
fn ler_csv(caminho: &Path) -> Result<Vec<RecordBatch>> {
    let schema = schema_vendas();
    let file = File::open(caminho).with_context(|| format!("abrindo {}", caminho.display()))?;
    let reader = ReaderBuilder::new(schema)
        .with_header(true)
        .build(file)
        .context("criando reader CSV")?;

    reader
        .collect::<Result<Vec<_>, _>>()
        .context("lendo batches")
}

/// Escreve uma lista de `RecordBatch` em um arquivo Parquet.
fn escrever_parquet(caminho: &Path, batches: &[RecordBatch]) -> Result<()> {
    if batches.is_empty() {
        anyhow::bail!("nenhum RecordBatch para escrever");
    }
    let schema = batches[0].schema();
    let file = File::create(caminho).with_context(|| format!("criando {}", caminho.display()))?;
    let mut writer = ArrowWriter::try_new(file, schema, None).context("criando ArrowWriter")?;

    for batch in batches {
        writer.write(batch).context("escrevendo batch")?;
    }
    writer.close().context("finalizando parquet")?;
    Ok(())
}

/// Lê um arquivo Parquet para uma lista de `RecordBatch`.
fn ler_parquet(caminho: &Path) -> Result<Vec<RecordBatch>> {
    let file = File::open(caminho).with_context(|| format!("abrindo {}", caminho.display()))?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .context("criando reader parquet")?
        .build()
        .context("build do reader parquet")?;

    reader
        .collect::<Result<Vec<_>, _>>()
        .context("lendo batches do parquet")
}

fn main() -> Result<()> {
    let entrada = Path::new("dados/vendas.csv");
    let saida = Path::new("dados/saida/vendas.parquet");
    let saida_parent = saida
        .parent()
        .ok_or_else(|| anyhow::anyhow!("caminho sem parent: {}", saida.display()))?;
    std::fs::create_dir_all(saida_parent).context("criando diretório de saída")?;

    let batches = ler_csv(entrada)?;
    println!("Lidos {} batches do CSV", batches.len());
    let total_csv: usize = batches.iter().map(|b| b.num_rows()).sum();
    println!("Total de linhas no CSV: {total_csv}");

    escrever_parquet(saida, &batches)?;
    println!("Parquet escrito em: {}", saida.display());

    let lidos = ler_parquet(saida)?;
    let total_parquet: usize = lidos.iter().map(|b| b.num_rows()).sum();
    println!("Total de linhas no Parquet: {total_parquet}");
    println!("Schema: {}", lidos[0].schema());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn csv_e_parquet_tem_mesmo_numero_de_linhas() {
        let batches_csv = ler_csv(Path::new("dados/vendas.csv")).expect("ok");
        let total_csv: usize = batches_csv.iter().map(|b| b.num_rows()).sum();

        let saida = env::temp_dir().join("vendas_test.parquet");
        escrever_parquet(&saida, &batches_csv).expect("ok");

        let batches_pq = ler_parquet(&saida).expect("ok");
        let total_pq: usize = batches_pq.iter().map(|b| b.num_rows()).sum();

        assert_eq!(total_csv, total_pq);
        std::fs::remove_file(saida).unwrap();
    }

    #[test]
    fn schema_tem_colunas_esperadas() {
        let schema = schema_vendas();
        let nomes: Vec<&str> = schema.fields.iter().map(|f| f.name().as_str()).collect();
        assert_eq!(
            nomes,
            vec![
                "id",
                "data",
                "categoria",
                "produto",
                "quantidade",
                "preco_unitario",
                "regiao",
                "cliente_id"
            ]
        );
    }
}
