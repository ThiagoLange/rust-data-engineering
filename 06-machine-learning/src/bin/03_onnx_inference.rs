//! Módulo 06 — Machine Learning
//! README: seção "Exemplo prático 2: Inferência ONNX"
//!
//! Carrega modelo ONNX exportado (ex: `models/classifier.onnx`) e roda
//! inferência em batch sobre dados do pipeline, integrando Polars para
//! pré-processamento. Se o modelo não existir, demonstra o fluxo com dados
//! sintéticos e avisa como exportar via Python (skl2onnx).

use anyhow::{Context, Result};
use clap::Parser;
use ndarray::{Array2, Axis};
use ort::session::Session;
use polars::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "onnx_inference")]
struct Args {
    /// Caminho do modelo ONNX
    #[arg(long, default_value = "models/classifier.onnx")]
    model: PathBuf,

    /// Batch size para inferência
    #[arg(long, default_value_t = 32)]
    batch_size: usize,
}

fn carregar_features(path: &Path, batch_size: usize) -> Result<Array2<f32>> {
    // Tenta carregar vendas.csv, senão gera sintético
    let df = if path.exists() {
        LazyCsvReader::new(path.to_string_lossy().as_ref().into())
            .with_has_header(true)
            .finish()
            .map_err(|e| anyhow::anyhow!("csv reader: {e}"))?
            .limit(batch_size as u32)
            .collect()
            .map_err(|e| anyhow::anyhow!("collect: {e}"))?
    } else {
        // Gera sintético direto como Array2 (evita DataFrame::new API)
        let mut arr = Array2::<f32>::zeros((batch_size, 2));
        for i in 0..batch_size {
            arr[[i, 0]] = i as f32;
            arr[[i, 1]] = i as f32 * 10.0;
        }
        return Ok(arr);
    };

    // Extrai quantidade e preco como f32 (robusto a u32/i32/i64)
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

    let n = df.height().min(batch_size);
    let mut arr = Array2::<f32>::zeros((n, 2));
    for i in 0..n {
        arr[[i, 0]] = qtd.get(i).unwrap_or(0) as f32;
        arr[[i, 1]] = preco.get(i).unwrap_or(0.0) as f32;
    }
    Ok(arr)
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("=== Inferência ONNX — Módulo 06 ===\n");
    println!("Modelo: {}", args.model.display());
    println!("Batch size: {}", args.batch_size);

    // Carrega features via Polars
    let csv = Path::new("../03-armazenamento-formatos/dados/vendas.csv");
    let features = carregar_features(csv, args.batch_size)?;
    println!("\nFeatures shape: {:?}", features.dim());
    println!(
        "Primeiras 3 linhas:\n{:?}",
        features.slice(ndarray::s![0..3.min(features.nrows()), ..])
    );

    // Tenta carregar modelo ONNX via ort
    if !args.model.exists() {
        println!("\n⚠ Modelo ONNX não encontrado em {}", args.model.display());
        println!("  Para gerar um modelo real:");
        println!("    1. Treine em Python com sklearn e exporte via skl2onnx:");
        println!("       python -c \"from sklearn.linear_model import LogisticRegression; ...; from skl2onnx import convert_sklearn; ...\"");
        println!(
            "    2. Ou use o modelo gerado por 01_linfa_classifier (linfa_model.json) e converta."
        );
        println!("\n  Demonstração com inferência simulada (sem ONNX):");
        // Simula inferência: threshold simples
        let preds: Vec<usize> = features
            .axis_iter(Axis(0))
            .map(|row| usize::from(row[0] * row[1] > 5000.0))
            .collect();
        println!(
            "  Predições simuladas (threshold qtd*preco>5000): {:?}",
            &preds[..5.min(preds.len())]
        );
        println!("\n✓ Fluxo Polars → ndarray → (ort) demonstrado com fallback sintético.");
        return Ok(());
    }

    // Inicializa ONNX Runtime (ort 2.0 requer init)
    let _ = ort::init().commit();

    println!("\nCarregando modelo ONNX...");
    let mut session = Session::builder()
        .context("session builder")?
        .commit_from_file(args.model.clone())
        .with_context(|| format!("carregando {}", args.model.display()))?;

    println!("Modelo carregado. Inputs: {:?}", session.inputs());
    println!("Outputs: {:?}", session.outputs());

    // Prepara input — assume modelo espera float32 com shape [batch, 2]
    let input_name = session.inputs()[0].name().to_string();
    println!("\nExecutando inferência...");
    // Converte para formato (shape, data) para evitar mismatch de ndarray version
    let shape = vec![features.nrows() as i64, features.ncols() as i64];
    #[allow(deprecated)]
    let data = features.into_raw_vec();
    let input_tensor = ort::value::Tensor::from_array((shape, data)).context("criando tensor")?;

    let outputs = session
        .run(ort::inputs![input_name.as_str() => input_tensor])
        .context("inferência")?;

    println!("Outputs: {:?}", outputs);
    // Tenta extrair predições
    if let Some((_, value)) = outputs.iter().next() {
        println!("Primeiro output extraído: {:?}", value);
    }

    println!("\n✓ Inferência ONNX concluída");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn carregar_features_sintetico() {
        // Usa path inexistente para forçar sintético
        let arr = carregar_features(Path::new("/tmp/nao_existe.csv"), 10).expect("ok");
        assert_eq!(arr.nrows(), 10);
        assert_eq!(arr.ncols(), 2);
    }

    #[test]
    fn features_contem_valores() {
        let arr = carregar_features(
            Path::new("../03-armazenamento-formatos/dados/vendas.csv"),
            5,
        )
        .expect("ok");
        assert_eq!(arr.ncols(), 2);
        assert!(arr.nrows() > 0);
    }
}
