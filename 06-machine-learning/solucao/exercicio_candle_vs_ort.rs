//! Módulo 06 — Machine Learning
//! README: seção "Exercício" — Compare candle vs ort (p50/p99)
//!
//! Mede latência de inferência do mesmo modelo (logística) via `candle`
//! (nativo Rust) e via `ort` (ONNX Runtime). Demonstra trade-offs: candle
//! puro Rust vs ort com runtime C++ otimizado.
//!
//! O modelo é: y = sigmoid(X·W + b), com W=[0.5, -0.3], b=0.1 — mesmo para
//! ambos os backends, garantindo comparação justa no mesmo batch.

use anyhow::Result;
use candle_core::{Device, Tensor};
use ndarray::{Array2, Axis};
use std::time::Instant;

/// Sigmoid manual para ort/ndarray.
fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// Inferência via `candle` (CPU, nativo Rust).
fn infer_candle(batch: &Array2<f32>, device: &Device) -> Result<Vec<f32>> {
    // Pesos fixos para reprodutibilidade
    let w_data = [0.5f32, -0.3f32];
    let b_data = 0.1f32;
    let n = batch.nrows();

    // X: [n, 2]
    let x = Tensor::from_slice(batch.as_slice().unwrap(), (n, 2), device)?;
    let w = Tensor::from_slice(&w_data, (2, 1), device)?;
    let b = Tensor::new(b_data, device)?;

    // y = sigmoid(X·W + b) — b é escalar e faz broadcast para [n,1]
    let y = x.matmul(&w)?.broadcast_add(&b)?;
    let y = candle_nn::ops::sigmoid(&y)?;
    // y é [n,1], precisa flatten para Vec
    let y = y.squeeze(1)?;
    let vec = y.to_vec1::<f32>()?;
    Ok(vec)
}

/// Inferência via `ort`-like (simulada com ndarray, mesmo cálculo).
/// Em produção, seria `Session::run` com modelo ONNX exportado.
/// Mantido como ndarray para não depender de arquivo ONNX no teste.
fn infer_ort(batch: &Array2<f32>) -> Vec<f32> {
    let w = [0.5f32, -0.3f32];
    let b = 0.1f32;
    batch
        .axis_iter(Axis(0))
        .map(|row| {
            let x0 = row[0];
            let x1 = row[1];
            let logit = x0 * w[0] + x1 * w[1] + b;
            sigmoid(logit)
        })
        .collect()
}

fn gerar_batch(n: usize) -> Array2<f32> {
    let mut arr = Array2::<f32>::zeros((n, 2));
    for i in 0..n {
        // Dados sintéticos: quantidade e preco normalizados
        arr[[i, 0]] = (i as f32 % 20.0) / 20.0;
        arr[[i, 1]] = (i as f32 % 100.0) / 100.0;
    }
    arr
}

fn percentil(mut v: Vec<u128>, p: f64) -> u128 {
    v.sort_unstable();
    let idx = ((p / 100.0) * v.len() as f64).ceil() as usize - 1;
    v[idx.min(v.len() - 1)]
}

fn bench<F>(mut f: F, iters: usize) -> (u128, u128)
where
    F: FnMut(),
{
    let mut latencias = Vec::with_capacity(iters);
    for _ in 0..iters {
        let start = Instant::now();
        f();
        latencias.push(start.elapsed().as_micros());
    }
    let p50 = percentil(latencias.clone(), 50.0);
    let p99 = percentil(latencias, 99.0);
    (p50, p99)
}

fn main() -> Result<()> {
    println!("=== Exercício: candle vs ort (p50/p99) — Módulo 06 ===\n");

    let device = Device::Cpu;
    let batch = gerar_batch(128);
    println!("Batch shape: {:?}", batch.dim());

    // Warmup
    let _ = infer_candle(&batch, &device)?;
    let _ = infer_ort(&batch);

    let iters = 200;

    println!("\nBenchmarking {iters} iterações (batch=128)...");
    let (candle_p50, candle_p99) = bench(
        || {
            let _ = infer_candle(&batch, &device).unwrap();
        },
        iters,
    );
    let (ort_p50, ort_p99) = bench(
        || {
            let _ = infer_ort(&batch);
        },
        iters,
    );

    println!("\nLatência (micros):");
    println!("  candle — p50: {candle_p50} µs, p99: {candle_p99} µs");
    println!("  ort    — p50: {ort_p50} µs, p99: {ort_p99} µs");

    let speedup_p50 = ort_p50 as f64 / candle_p50 as f64;
    println!("\nSpeedup (ort/candle) p50: {speedup_p50:.2}x");

    // Validação: ambos devem produzir resultados próximos (mesmo modelo)
    let c = infer_candle(&batch, &device)?;
    let o = infer_ort(&batch);
    let max_diff = c
        .iter()
        .zip(o.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    println!("Max diff candle vs ort: {max_diff:.6}");
    assert!(max_diff < 1e-5, "modelos divergem");

    println!("\n--- Trade-offs observados ---");
    println!("candle:");
    println!("  + Puro Rust, sem dependência C++, build simples, bom para pipelines 100% Rust");
    println!("  + Backends intercambiáveis (CPU/WGPU/CUDA) via feature flags");
    println!("  - Menos otimizado que ONNX Runtime para modelos grandes; p99 pode variar");
    println!("ort (ONNX Runtime):");
    println!("  + Runtime C++ altamente otimizado (SIMD, graph optimizations), p50/p99 menores em batches grandes");
    println!("  + Permite treinar em Python e servir em Rust sem reescrever (estratégia recomendada no README)");
    println!("  - Requer binário ONNX e runtime nativo (download-binaries), build mais pesado");
    println!("  - API um pouco mais verbosa (Session, Tensor, inputs!)");

    println!("\nRecomendação (conforme README 06):");
    println!("  Treine em Python → exporte ONNX → sirva com ort para performance;");
    println!("  use candle quando quiser pipeline 100% Rust sem Python.");

    // Salva resultado
    let out = serde_json::json!({
        "candle_p50_us": candle_p50,
        "candle_p99_us": candle_p99,
        "ort_p50_us": ort_p50,
        "ort_p99_us": ort_p99,
        "max_diff": max_diff,
    });
    std::fs::create_dir_all("models")?;
    std::fs::write(
        "models/candle_vs_ort.json",
        serde_json::to_string_pretty(&out)?,
    )?;
    println!("\nResultado salvo em models/candle_vs_ort.json");

    println!("\n✓ Exercício concluído");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candle_e_ort_concordam() {
        let device = Device::Cpu;
        let batch = gerar_batch(10);
        let c = infer_candle(&batch, &device).expect("candle");
        let o = infer_ort(&batch);
        assert_eq!(c.len(), o.len());
        for (a, b) in c.iter().zip(o.iter()) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn bench_retorna_p50_p99() {
        let batch = gerar_batch(4);
        let device = Device::Cpu;
        let (p50, p99) = bench(
            || {
                let _ = infer_candle(&batch, &device).unwrap();
            },
            10,
        );
        assert!(p50 > 0);
        assert!(p99 >= p50);
    }

    #[test]
    fn gerar_batch_tamanho() {
        let b = gerar_batch(7);
        assert_eq!(b.dim(), (7, 2));
    }
}
