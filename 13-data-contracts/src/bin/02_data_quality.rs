#![deny(warnings)]
use anyhow::Result;
use polars::prelude::*;

#[derive(Debug)]
struct SuiteChecks {
    nulos: Vec<String>,
    fora_range: Vec<(String, f64, f64)>,
    duplicados: Vec<String>,
    total: usize,
}

impl SuiteChecks {
    fn validar_df(df: &DataFrame) -> Self {
        let mut nulos = Vec::new();
        let mut fora_range = Vec::new();
        let mut duplicados = Vec::new();

        for (nome, _) in df.schema().iter() {
            let col = df.column(nome).unwrap();

            if let Ok(ser) = col.f64() {
                let valores: Vec<f64> = ser.iter().map(|v| v.unwrap_or(0.0)).collect();

                let null_count = valores.iter().filter(|v| v.is_nan()).count();
                if null_count > 0 {
                    nulos.push(format!("{}: {} nulos", nome, null_count));
                }

                let negativos: Vec<f64> = valores.iter().filter(|v| **v < 0.0).copied().collect();
                if !negativos.is_empty() {
                    let min_val = negativos.iter().fold(f64::INFINITY, |a, b| a.min(*b));
                    let max_val = negativos.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b));
                    fora_range.push((nome.to_string(), min_val, max_val));
                }

                let mut s = valores.clone();
                s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                s.dedup();
                if s.len() < valores.len() {
                    duplicados.push(nome.to_string());
                }
            }
        }

        SuiteChecks {
            nulos,
            fora_range,
            duplicados,
            total: df.height(),
        }
    }

    fn reportar(&self) -> String {
        let mut partes = Vec::new();
        for n in &self.nulos {
            partes.push(format!("⚠ {}", n));
        }
        for (col, min, max) in &self.fora_range {
            partes.push(format!("⚠ {} fora de range: {:.2}–{:.2}", col, min, max));
        }
        for d in &self.duplicados {
            partes.push(format!("⚠ {} tem duplicados", d));
        }
        if partes.is_empty() {
            "✓ todos os checks passaram".to_string()
        } else {
            partes.join("\n")
        }
    }
}

fn main() -> Result<()> {
    println!("=== Suite de qualidade de dados ===");

    let df = DataFrame::new(
        5,
        vec![
            Column::new("id".into(), &[1i64, 2, 3, 4, 5]),
            Column::new("valor".into(), &[100.0, -10.0, 200.0, 300.0, 400.0]),
            Column::new("produto".into(), &["A", "B", "C", "D", "E"]),
        ],
    )?;

    let checks = SuiteChecks::validar_df(&df);
    println!("total de registros: {}", checks.total);
    println!("{}", checks.reportar());

    let df2 = DataFrame::new(
        3,
        vec![
            Column::new("id".into(), &[1i64, 2, 3]),
            Column::new("valor".into(), &[100.0, f64::NAN, 200.0]),
            Column::new("produto".into(), &["A", "B", "C"]),
        ],
    )?;

    let checks2 = SuiteChecks::validar_df(&df2);
    println!("\n--- dados com NAN ---");
    println!("{}", checks2.reportar());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nulos_detectados() {
        let df = DataFrame::new(
            2,
            vec![
                Column::new("id".into(), &[1i64, 2]),
                Column::new("valor".into(), &[100.0, f64::NAN]),
            ],
        )
        .unwrap();

        let checks = SuiteChecks::validar_df(&df);
        assert!(!checks.nulos.is_empty());
        assert!(checks.nulos.iter().any(|s| s.contains("nulos")));
    }

    #[test]
    fn positives_only_no_duplicados() {
        let df = DataFrame::new(
            3,
            vec![
                Column::new("id".into(), &[1i64, 2, 3]),
                Column::new("valor".into(), &[100.0, 200.0, 300.0]),
            ],
        )
        .unwrap();

        let checks = SuiteChecks::validar_df(&df);
        assert!(checks.duplicados.is_empty());
        assert!(checks.nulos.is_empty());
    }
}
