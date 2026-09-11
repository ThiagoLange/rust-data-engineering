//! Solução do exercício: segunda `#[pyfunction]` que resume transações por região.
//!
//! Estende o exemplo com `Transaction`/`summarize_transactions` e o teste
//! de round-trip serde sob GIL (`Python::with_gil` — `acquire_gil` foi
//! removido no PyO3 0.23).

use anyhow::Result;
use polars::prelude::*;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Deserialize)]
struct InputData {
    vendas: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AggregationResult {
    count: usize,
    total: f64,
    average: f64,
    min: f64,
    max: f64,
}

#[derive(Error, Debug)]
enum PipelineError {
    #[error("dados vazios")]
    EmptyData,
    #[error("parse erro: {0}")]
    Parse(#[from] serde_json::Error),
}

fn aggregate_values(values: &[f64]) -> Result<AggregationResult> {
    if values.is_empty() {
        return Err(PipelineError::EmptyData.into());
    }
    let df = DataFrame::new(values.len(), vec![Column::new("valor".into(), values)])?;
    let summary: Vec<f64> = df
        .column("valor")
        .map_err(|e| anyhow::anyhow!("coluna valor: {e}"))?
        .f64()
        .map_err(|e| anyhow::anyhow!("cast f64: {e}"))?
        .iter()
        .flatten()
        .collect();

    Ok(AggregationResult {
        count: summary.len(),
        total: summary.iter().sum(),
        average: summary.iter().sum::<f64>() / summary.len() as f64,
        min: summary.iter().copied().fold(f64::INFINITY, f64::min),
        max: summary.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    })
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Transaction {
    produto: String,
    valor: f64,
    regiao: String,
}

#[derive(Debug, Serialize)]
struct TransactionSummary {
    total_transacoes: usize,
    por_regiao: HashMap<String, f64>,
    maior_transacao: Option<Transaction>,
}

fn summarize_transactions(json: &str) -> Result<TransactionSummary> {
    let txns: Vec<Transaction> = serde_json::from_str(json)?;
    let mut por_regiao: HashMap<String, f64> = HashMap::new();
    for t in &txns {
        *por_regiao.entry(t.regiao.clone()).or_insert(0.0) += t.valor;
    }
    let maior = txns
        .iter()
        .cloned()
        .reduce(|a, b| if a.valor > b.valor { a } else { b });
    Ok(TransactionSummary {
        total_transacoes: txns.len(),
        por_regiao,
        maior_transacao: maior,
    })
}

fn main() -> Result<()> {
    let json = r#"{"vendas":[100.0,200.0,300.0]}"#;
    let input: InputData = serde_json::from_str(json)?;
    let result = aggregate_values(&input.vendas)?;
    println!("resultado: {}", serde_json::to_string_pretty(&result)?);

    let txns = r#"[{"produto":"A","valor":50.0,"regiao":"Norte"},{"produto":"B","valor":150.0,"regiao":"Sul"},{"produto":"C","valor":100.0,"regiao":"Norte"}]"#;
    let summary = summarize_transactions(txns)?;
    println!("transacoes: {}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

#[pyfunction]
fn processar_dados(json: &str) -> PyResult<String> {
    let input: InputData = serde_json::from_str(json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
    let result = aggregate_values(&input.vendas)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    serde_json::to_string(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

#[pyfunction]
fn resumir_transacoes(json: &str) -> PyResult<String> {
    let result = summarize_transactions(json)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;
    serde_json::to_string(&result)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))
}

#[pymodule]
fn rust_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(processar_dados, m)?)?;
    m.add_function(wrap_pyfunction!(resumir_transacoes, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agrega_valores() {
        let result = aggregate_values(&[100.0, 200.0, 300.0]).unwrap();
        assert_eq!(result.count, 3);
        assert_eq!(result.total, 600.0);
        assert_eq!(result.average, 200.0);
    }

    #[test]
    fn rejeita_lista_vazia() {
        assert!(aggregate_values(&[]).is_err());
    }

    #[test]
    fn resumo_transacoes_por_regiao() {
        let txns = r#"[{"produto":"A","valor":50.0,"regiao":"Norte"},{"produto":"B","valor":150.0,"regiao":"Sul"},{"produto":"C","valor":100.0,"regiao":"Norte"}]"#;
        let summary = summarize_transactions(txns).unwrap();
        assert_eq!(summary.total_transacoes, 3);
        assert_eq!(summary.por_regiao.get("Norte"), Some(&150.0));
        assert_eq!(summary.maior_transacao.as_ref().unwrap().valor, 150.0);
    }

    #[test]
    fn py_function_serde_roundtrip() {
        Python::with_gil(|_| {
            let out = processar_dados(r#"{"vendas":[10.0,20.0]}"#).unwrap();
            let parsed: AggregationResult = serde_json::from_str(&out).unwrap();
            assert_eq!(parsed.total, 30.0);
        });
    }
}
