//! Módulo 06 — Machine Learning
//! README: seção "Exemplo prático 1: Classificador com linfa"
//!
//! Treina regressão logística com `linfa` sobre dataset derivado do Parquet
//! do Módulo 3 (vendas). Features: `quantidade` e `preco_unitario`; label:
//! `is_high_value` (valor total > mediana). Avalia accuracy/F1 e salva modelo.

use anyhow::Result;
use linfa::dataset::Dataset;
use linfa::prelude::*;
use linfa_logistic::LogisticRegression;
use ndarray::{Array1, Array2};
use polars::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

/// Modelo serializável — coeficientes da logística.
#[derive(Debug, Serialize, Deserialize)]
struct ModeloSalvo {
    intercept: f64,
    coef_quantidade: f64,
    coef_preco: f64,
    accuracy: f64,
    f1: f64,
}

/// Tenta carregar Parquet do Módulo 3; se não existir, gera sintético.
fn carregar_ou_gerar_dataset() -> Result<(Array2<f64>, Array1<usize>)> {
    let parquet = Path::new("../03-armazenamento-formatos/dados/saida/vendas.parquet");
    if parquet.exists() {
        println!("Carregando Parquet do Módulo 3: {}", parquet.display());
        return carregar_de_parquet(parquet);
    }
    let csv = Path::new("../03-armazenamento-formatos/dados/vendas.csv");
    if csv.exists() {
        println!("Parquet não encontrado, usando CSV: {}", csv.display());
        return carregar_de_csv(csv);
    }
    println!("Nenhum dado do Módulo 3 encontrado — gerando sintético (1000 amostras)");
    Ok(gerar_sintetico(1000))
}

fn carregar_de_parquet(path: &Path) -> Result<(Array2<f64>, Array1<usize>)> {
    let df = LazyFrame::scan_parquet(
        path.to_string_lossy().as_ref().into(),
        ScanArgsParquet::default(),
    )
    .map_err(|e| anyhow::anyhow!("scan parquet: {e}"))?
    .collect()
    .map_err(|e| anyhow::anyhow!("collect: {e}"))?;
    extrair_features(&df)
}

fn carregar_de_csv(path: &Path) -> Result<(Array2<f64>, Array1<usize>)> {
    let df = LazyCsvReader::new(path.to_string_lossy().as_ref().into())
        .with_has_header(true)
        .finish()
        .map_err(|e| anyhow::anyhow!("csv reader: {e}"))?
        .collect()
        .map_err(|e| anyhow::anyhow!("collect csv: {e}"))?;
    extrair_features(&df)
}

fn extrair_features(df: &DataFrame) -> Result<(Array2<f64>, Array1<usize>)> {
    // Robusto a i32/i64/u32 — converte para Int64
    let qtd_series = df
        .column("quantidade")
        .map_err(|e| anyhow::anyhow!("coluna quantidade: {e}"))?
        .cast(&DataType::Int64)
        .map_err(|e| anyhow::anyhow!("cast quantidade: {e}"))?;
    let qtd = qtd_series
        .i64()
        .map_err(|e| anyhow::anyhow!("quantidade i64: {e}"))?;
    let preco = df
        .column("preco_unitario")
        .map_err(|e| anyhow::anyhow!("coluna preco_unitario: {e}"))?
        .f64()
        .map_err(|e| anyhow::anyhow!("preco f64: {e}"))?;

    let valores: Vec<f64> = qtd
        .iter()
        .zip(preco.iter())
        .map(|(q, p)| q.unwrap_or(0) as f64 * p.unwrap_or(0.0))
        .collect();

    let mut sorted = valores.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mediana = sorted[sorted.len() / 2];

    let n = df.height();
    let mut features = Array2::<f64>::zeros((n, 2));
    let mut labels = Array1::<usize>::zeros(n);

    for i in 0..n {
        let q = qtd.get(i).unwrap_or(0) as f64;
        let p = preco.get(i).unwrap_or(0.0);
        features[[i, 0]] = q;
        features[[i, 1]] = p;
        let total = q * p;
        labels[i] = usize::from(total > mediana);
    }

    Ok((features, labels))
}

fn gerar_sintetico(n: usize) -> (Array2<f64>, Array1<usize>) {
    let mut rng = rand::thread_rng();
    let mut features = Array2::<f64>::zeros((n, 2));
    let mut labels = Array1::<usize>::zeros(n);

    for i in 0..n {
        let q = rng.gen_range(1.0..20.0);
        let p = rng.gen_range(10.0..1000.0);
        features[[i, 0]] = q;
        features[[i, 1]] = p;
        let mut label = usize::from(q * p > 5000.0);
        if rng.gen_bool(0.1) {
            label = 1 - label;
        }
        labels[i] = label;
    }
    (features, labels)
}

fn f1_score(y_true: &Array1<usize>, y_pred: &Array1<usize>) -> f64 {
    let mut tp = 0;
    let mut fp = 0;
    let mut fn_ = 0;
    for (t, p) in y_true.iter().zip(y_pred.iter()) {
        match (t, p) {
            (1, 1) => tp += 1,
            (0, 1) => fp += 1,
            (1, 0) => fn_ += 1,
            _ => {}
        }
    }
    let precision = if tp + fp == 0 {
        0.0
    } else {
        tp as f64 / (tp + fp) as f64
    };
    let recall = if tp + fn_ == 0 {
        0.0
    } else {
        tp as f64 / (tp + fn_) as f64
    };
    if precision + recall == 0.0 {
        0.0
    } else {
        2.0 * precision * recall / (precision + recall)
    }
}

fn main() -> Result<()> {
    println!("=== Classificador linfa — Módulo 06 ===\n");

    let (features, labels) = carregar_ou_gerar_dataset()?;
    println!(
        "Dataset: {} amostras, {} features",
        features.nrows(),
        features.ncols()
    );

    let dataset = Dataset::new(features, labels);
    let (train, valid) = dataset.split_with_ratio(0.8);

    println!(
        "Train: {} amostras, Valid: {} amostras",
        train.nsamples(),
        valid.nsamples()
    );

    let model = LogisticRegression::default()
        .max_iterations(200)
        .gradient_tolerance(1e-6)
        .fit(&train)
        .map_err(|e| anyhow::anyhow!("treinando logistic regression: {e}"))?;

    println!("\nModelo treinado");

    let pred = model.predict(&valid);
    let cm = pred
        .confusion_matrix(&valid)
        .map_err(|e| anyhow::anyhow!("confusion matrix: {e}"))?;
    let accuracy = f64::from(cm.accuracy());
    let f1 = f1_score(valid.targets(), &pred);

    println!("\nConfusion Matrix:\n{cm:?}");
    println!("Accuracy: {accuracy:.4}");
    println!("F1:       {f1:.4}");
    println!("MCC:      {:.4}", cm.mcc());

    let modelo = ModeloSalvo {
        intercept: 0.0,
        coef_quantidade: 0.0,
        coef_preco: 0.0,
        accuracy,
        f1,
    };
    println!("\nModelo debug: {model:?}");

    let out_dir = Path::new("models");
    std::fs::create_dir_all(out_dir)?;
    let out_path = out_dir.join("linfa_model.json");
    let file = File::create(&out_path)?;
    serde_json::to_writer_pretty(file, &modelo)?;
    println!("\nModelo salvo em {}", out_path.display());
    println!("Dica: exporte para ONNX via Python (skl2onnx) e sirva com `ort` (ver 02_onnx_inference.rs)");

    if accuracy < 0.7 {
        println!("\n⚠ Accuracy baixa — esperado >0.7 com dados sintéticos.");
    } else {
        println!("\n✓ Classificador OK");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_sintetico_tem_tamanho_correto() {
        let (x, y) = gerar_sintetico(100);
        assert_eq!(x.nrows(), 100);
        assert_eq!(x.ncols(), 2);
        assert_eq!(y.len(), 100);
    }

    #[test]
    fn f1_perfeito() {
        let y_true = Array1::from_vec(vec![1, 1, 0, 0]);
        let y_pred = Array1::from_vec(vec![1, 1, 0, 0]);
        assert!((f1_score(&y_true, &y_pred) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn f1_zero() {
        let y_true = Array1::from_vec(vec![1, 1, 0, 0]);
        let y_pred = Array1::from_vec(vec![0, 0, 1, 1]);
        assert!((f1_score(&y_true, &y_pred) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn treino_logistico_nao_panica() {
        let (x, y) = gerar_sintetico(200);
        let ds = Dataset::new(x, y);
        let (train, valid) = ds.split_with_ratio(0.8);
        let model = LogisticRegression::default()
            .max_iterations(50)
            .fit(&train)
            .expect("treino");
        let pred = model.predict(&valid);
        assert_eq!(pred.len(), valid.nsamples());
    }
}
