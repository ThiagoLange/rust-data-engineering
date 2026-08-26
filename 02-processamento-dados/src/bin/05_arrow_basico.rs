//! Módulo 02 — Processamento de Dados
//! README: seção "Conteúdo", item 3 — Arrow-rs
//!
//! Apache Arrow é um formato colunar in-memory. Polars e DataFusion são
//! construídos sobre ele. Entender Arrow no nível básico ajuda a entender
//! por que operações colunares são rápidas e por que dados podem ser
//! compartilhados entre linguagens e bibliotecas sem cópia (zero-copy).
//!
//! Conceitos fundamentais:
//! - `Schema`: descreve os nomes e tipos das colunas.
//! - `ArrayRef`: um vetor tipado de valores (Int32Array, StringArray, etc.).
//! - `RecordBatch`: um "quadro" de dados composto por um schema e uma lista
//!   de arrays, todos do mesmo comprimento.

use arrow::array::{ArrayRef, Float64Array, Int32Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

/// Cria um RecordBatch de exemplo representando vendas.
fn criar_batch_vendas() -> anyhow::Result<RecordBatch> {
    let schema = Schema::new(vec![
        Field::new("id", DataType::Int32, false),
        Field::new("produto", DataType::Utf8, false),
        Field::new("quantidade", DataType::Int32, false),
        Field::new("preco", DataType::Float64, false),
    ]);

    let ids = Int32Array::from(vec![1, 2, 3, 4]);
    let produtos = StringArray::from(vec!["Teclado", "Mouse", "Monitor", "Webcam"]);
    let quantidades = Int32Array::from(vec![2, 5, 1, 3]);
    let precos = Float64Array::from(vec![349.90, 89.50, 1899.00, 199.90]);

    RecordBatch::try_new(
        Arc::new(schema),
        vec![
            Arc::new(ids) as ArrayRef,
            Arc::new(produtos) as ArrayRef,
            Arc::new(quantidades) as ArrayRef,
            Arc::new(precos) as ArrayRef,
        ],
    )
    .map_err(Into::into)
}

/// Calcula o valor total (quantidade * preço) para cada linha do batch.
fn calcular_total(batch: &RecordBatch) -> anyhow::Result<Float64Array> {
    let quantidades = batch
        .column_by_name("quantidade")
        .ok_or_else(|| anyhow::anyhow!("coluna 'quantidade' não encontrada"))?
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| anyhow::anyhow!("'quantidade' não é Int32Array"))?;

    let precos = batch
        .column_by_name("preco")
        .ok_or_else(|| anyhow::anyhow!("coluna 'preco' não encontrada"))?
        .as_any()
        .downcast_ref::<Float64Array>()
        .ok_or_else(|| anyhow::anyhow!("'preco' não é Float64Array"))?;

    let total: Vec<f64> = quantidades
        .iter()
        .zip(precos.iter())
        .map(|(q, p)| q.unwrap_or(0) as f64 * p.unwrap_or(0.0))
        .collect();

    Ok(Float64Array::from(total))
}

fn main() -> anyhow::Result<()> {
    let batch = criar_batch_vendas()?;

    println!("=== Schema ===");
    println!("{}", batch.schema());

    println!("\n=== RecordBatch ===");
    println!("{batch:?}");

    println!("\n=== Valor total por linha ===");
    let total = calcular_total(&batch)?;
    for (i, valor) in total.iter().enumerate() {
        println!("linha {i}: {:.2}", valor.unwrap_or(0.0));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_tem_quatro_linhas_e_quatro_colunas() {
        let batch = criar_batch_vendas().expect("ok");
        assert_eq!(batch.num_rows(), 4);
        assert_eq!(batch.num_columns(), 4);
    }

    #[test]
    fn total_calculado_esta_correto() {
        let batch = criar_batch_vendas().expect("ok");
        let total = calcular_total(&batch).expect("ok");
        // 2 * 349.90 = 699.80
        assert!((total.value(0) - 699.80).abs() < 0.001);
        // 5 * 89.50 = 447.50
        assert!((total.value(1) - 447.50).abs() < 0.001);
    }
}
