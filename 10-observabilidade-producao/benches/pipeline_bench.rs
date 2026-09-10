//! Módulo 10 — Benchmark com criterion: com vs sem paralelismo.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use polars::prelude::*;

fn gerar_df(n: usize) -> DataFrame {
    let metrica: Vec<f64> = (0..n)
        .map(|i| 100.0 + (i as f64 * 0.3).sin() * 10.0)
        .collect();
    DataFrame::new(n, vec![Column::new("metrica".into(), metrica)]).unwrap()
}

fn sequencial(df: &DataFrame) -> f64 {
    let met = df.column("metrica").unwrap().f64().unwrap();
    met.iter().map(|v| v.unwrap_or(0.0) * 1.1).sum()
}

fn com_paralelismo(df: &DataFrame) -> f64 {
    // Paralelismo via Polars lazy (usa thread pool interno)
    df.clone()
        .lazy()
        .with_column((col("metrica") * lit(1.1)).alias("ajustada"))
        .select([col("ajustada").sum()])
        .collect()
        .unwrap()
        .column("ajustada")
        .unwrap()
        .f64()
        .unwrap()
        .get(0)
        .unwrap_or(0.0)
}

fn bench_pipeline(c: &mut Criterion) {
    let df = gerar_df(10_000);
    c.bench_function("sequencial_iter", |b| b.iter(|| sequencial(black_box(&df))));
    c.bench_function("lazy_paralelo", |b| {
        b.iter(|| com_paralelismo(black_box(&df)))
    });
}

criterion_group!(benches, bench_pipeline);
criterion_main!(benches);
