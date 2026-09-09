//! Utilitários compartilhados entre os exemplos de Parquet.
//!
//! Centraliza o schema e a leitura do CSV de vendas para evitar repetição.

use anyhow::{Context, Result};
use arrow::record_batch::RecordBatch;
use arrow_csv::ReaderBuilder;
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

/// Schema esperado do CSV de vendas.
pub fn schema_vendas() -> SchemaRef {
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
pub fn ler_csv(caminho: &Path, schema: SchemaRef) -> Result<Vec<RecordBatch>> {
    let file = File::open(caminho).with_context(|| format!("abrindo {}", caminho.display()))?;
    let reader = ReaderBuilder::new(schema)
        .with_header(true)
        .build(file)
        .context("criando reader CSV")?;

    reader
        .collect::<Result<Vec<_>, _>>()
        .context("lendo batches")
}
