//! Módulo 06 — Machine Learning
//! README: seção "Conteúdo" item 1 — Regressão linear com linfa
//!
//! Treina regressão linear (`linfa-linear`) sobre dados sintéticos e sobre
//! vendas (preco vs quantidade). Avalia RMSE/R² e salva coeficientes.

use anyhow::Result;
use linfa::dataset::Dataset;
use linfa::prelude::*;
use linfa_linear::LinearRegression;
use ndarray::{Array1, Array2};
use polars::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct ModeloLinear {
    intercept: f64,
    coef: Vec<f64>,
    rmse: f64,
    r2: f64,
}

fn gerar_linear(n: usize) -> (Array2<f64>, Array1<f64>) {
    let mut rng = rand::thread_rng();
    let mut x = Array2::<f64>::zeros((n, 1));
    let mut y = Array1::<f64>::zeros(n);
    for i in 0..n {
        let xi = rng.gen_range(0.0..10.0);
        let noise: f64 = rng.gen_range(-1.0..1.0);
        let yi = 2.5 * xi + 3.0 + noise;
        x[[i, 0]] = xi;
        y[i] = yi;
    }
    (x, y)
}

fn carregar_vendas_regressao() -> Result<(Array2<f64>, Array1<f64>)> {
    let csv = Path::new("../03-armazenamento-formatos/dados/vendas.csv");
    if !csv.exists() {
        println!("CSV não encontrado, usando sintético");
        return Ok(gerar_linear(500));
    }
    let df = LazyCsvReader::new(csv.to_string_lossy().as_ref().into())
        .with_has_header(true)
        .finish()
        .map_err(|e| anyhow::anyhow!("csv reader: {e}"))?
        .collect()
        .map_err(|e| anyhow::anyhow!("collect: {e}"))?;

    // Regressão: prever preco_unitario a partir de quantidade (robusto a i32/i64/u32)
    let qtd_series = df
        .column("quantidade")
        .map_err(|e| anyhow::anyhow!("quantidade: {e}"))?
        .cast(&DataType::Int64)
        .map_err(|e| anyhow::anyhow!("cast quantidade: {e}"))?;
    let qtd = qtd_series
        .i64()
        .map_err(|e| anyhow::anyhow!("quantidade i64: {e}"))?;
    let preco = df
        .column("preco_unitario")
        .map_err(|e| anyhow::anyhow!("preco: {e}"))?
        .f64()
        .map_err(|e| anyhow::anyhow!("f64: {e}"))?;

    let n = df.height();
    let mut x = Array2::<f64>::zeros((n, 1));
    let mut y = Array1::<f64>::zeros(n);
    for i in 0..n {
        let q = qtd.get(i).unwrap_or(0) as f64;
        let p = preco.get(i).unwrap_or(0.0);
        x[[i, 0]] = q;
        y[i] = p;
    }
    Ok((x, y))
}

fn rmse(y_true: &Array1<f64>, y_pred: &Array1<f64>) -> f64 {
    let n = y_true.len() as f64;
    let sum_sq: f64 = y_true
        .iter()
        .zip(y_pred.iter())
        .map(|(t, p)| (t - p).powi(2))
        .sum();
    (sum_sq / n).sqrt()
}

fn r2_score(y_true: &Array1<f64>, y_pred: &Array1<f64>) -> f64 {
    let mean = y_true.iter().sum::<f64>() / y_true.len() as f64;
    let ss_tot: f64 = y_true.iter().map(|t| (t - mean).powi(2)).sum();
    let ss_res: f64 = y_true
        .iter()
        .zip(y_pred.iter())
        .map(|(t, p)| (t - p).powi(2))
        .sum();
    if ss_tot == 0.0 {
        0.0
    } else {
        1.0 - ss_res / ss_tot
    }
}

fn main() -> Result<()> {
    println!("=== Regressão Linear — Módulo 06 ===\n");

    // Sintético (didático) — y = 2.5 x + 3 + ruído
    println!("--- Sintético: y = 2.5 x + 3 ---");
    let (x_syn, y_syn) = gerar_linear(500);
    let ds_syn = Dataset::new(x_syn, y_syn);
    let (train_syn, valid_syn) = ds_syn.split_with_ratio(0.8);
    let model_syn: linfa_linear::FittedLinearRegression<f64> = LinearRegression::default()
        .fit(&train_syn)
        .map_err(|e| anyhow::anyhow!("fit: {e}"))?;
    let pred_syn = model_syn.predict(&valid_syn);
    println!(
        "Sintético RMSE: {:.4}, R2: {:.4}",
        rmse(valid_syn.targets(), &pred_syn),
        r2_score(valid_syn.targets(), &pred_syn)
    );
    println!("Params sintético: {:?}", model_syn.params());

    // Vendas reais
    println!("\n--- Vendas (quantidade -> preco_unitario) ---");
    let (x, y) = carregar_vendas_regressao()?;
    println!("Dataset vendas: {} amostras", x.nrows());
    let ds = Dataset::new(x, y);
    let (train, valid) = ds.split_with_ratio(0.8);
    let model = LinearRegression::default()
        .fit(&train)
        .map_err(|e| anyhow::anyhow!("fit vendas: {e}"))?;
    let pred = model.predict(&valid);
    let rmse_v = rmse(valid.targets(), &pred);
    let r2_v = r2_score(valid.targets(), &pred);
    println!("Vendas RMSE: {rmse_v:.4}, R2: {r2_v:.4}");
    println!("Params vendas: {:?}", model.params());
    println!("Intercept: {:?}", model.intercept());

    let out_dir = Path::new("models");
    std::fs::create_dir_all(out_dir)?;
    let modelo = ModeloLinear {
        intercept: model.intercept(),
        coef: model.params().to_vec(),
        rmse: rmse_v,
        r2: r2_v,
    };
    let file = File::create(out_dir.join("linear_model.json"))?;
    serde_json::to_writer_pretty(file, &modelo)?;
    println!("\nModelo salvo em models/linear_model.json");
    println!("✓ Regressão linear OK");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gera_linear_tamanho() {
        let (x, y) = gerar_linear(50);
        assert_eq!(x.nrows(), 50);
        assert_eq!(y.len(), 50);
    }

    #[test]
    fn rmse_perfeito_zero() {
        let y = Array1::from_vec(vec![1.0, 2.0, 3.0]);
        assert!((rmse(&y, &y) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn treino_linear_nao_panica() {
        let (x, y) = gerar_linear(100);
        let ds = Dataset::new(x, y);
        let (train, valid) = ds.split_with_ratio(0.8);
        let m = LinearRegression::default().fit(&train).expect("fit");
        let p = m.predict(&valid);
        assert_eq!(p.len(), valid.nsamples());
    }
}
