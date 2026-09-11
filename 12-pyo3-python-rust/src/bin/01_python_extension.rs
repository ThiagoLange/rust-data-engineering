//! Extensão Python em Rust: agregação de vendas via Polars, exposta com PyO3.
//!
//! Seção correspondente no README: O que implementar → `exercicio_python_extension`.
//!
//! Roda como binário puro (`cargo run`) e como módulo Python (`maturin develop`
/// depois `import rust_module`).

use anyhow::Result;
use polars::prelude::*;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct InputData {
    vendas: Vec<f64>,
}

#[derive(Debug, Serialize)]
struct AggregationResult {
    count: usize,
    total: f64,
    average: f64,
    min: f64,
    max: f64,
}

fn aggregate_values(values: &[f64]) -> Result<AggregationResult> {
    if values.is_empty() {
        anyhow::bail!("lista vazia");
    }
    // Polars como motor de agregação (mesmo para slices pequenos, demonstra o padrão).
    let df = DataFrame::new(
        values.len(),
        vec![Column::new("valor".into(), values)],
    )?;
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

fn main() -> Result<()> {
    let json = r#"{"vendas":[100.0,200.0,300.0]}"#;
    let input: InputData = serde_json::from_str(json)?;
    let result = aggregate_values(&input.vendas)?;
    println!("resultado: {}", serde_json::to_string_pretty(&result)?);
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

#[pymodule]
fn rust_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(processar_dados, m)?)?;
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
    fn pyfunction_com_gil() {
        // `processar_dados` cria `PyErr` no erro: exige GIL mesmo no caminho feliz.
        Python::with_gil(|_| {
            let out = processar_dados(r#"{"vendas":[10.0,20.0]}"#).unwrap();
            assert!(out.contains("30.0"));
        });
    }
}