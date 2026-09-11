#![deny(warnings)]
use std::collections::HashMap;

use anyhow::{bail, Result};
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
enum ValidacaoError {
    #[error("tipo incompatível em {campo}: esperado {esperado}, encontrado {encontrado}")]
    TipoIncompativel {
        campo: String,
        esperado: String,
        encontrado: String,
    },
    #[error("valor nulo em {0}")]
    ValorNulo(String),
    #[error("valor fora de range em {campo}: {valor}")]
    ValorForaDeRange { campo: String, valor: f64 },
}

#[derive(Debug, Deserialize, Serialize)]
struct Contrato {
    schema: SchemaInfo,
    constraints: HashMap<String, Constraint>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct SchemaInfo {
    nome: String,
    versao: u32,
    campos: Vec<CampoSchema>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct CampoSchema {
    nome: String,
    tipo: String,
    required: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Constraint {
    unique: Option<bool>,
    required: Option<bool>,
    min: Option<f64>,
    max: Option<f64>,
}

fn parse_contrato(path: &str) -> Result<Contrato> {
    let c = std::fs::read_to_string(path)?;
    let contrato: Contrato = serde_json::from_str(&c)?;
    Ok(contrato)
}

fn validar(df: &DataFrame, contrato: &Contrato) -> Result<()> {
    for campo in &contrato.schema.campos {
        let col = df
            .column(&campo.nome)
            .map_err(|e| ValidacaoError::TipoIncompativel {
                campo: campo.nome.clone(),
                esperado: campo.tipo.clone(),
                encontrado: format!("coluna não encontrada: {}", e),
            })?;

        if campo.required {
            if let Ok(ser) = col.f64() {
                let has_null: bool = ser.iter().any(|v| v.is_none() || v.unwrap().is_nan());
                if has_null {
                    bail!(ValidacaoError::ValorNulo(campo.nome.clone()));
                }
            }
            if let Ok(ser) = col.i64() {
                let has_null: bool = ser.iter().any(|v| v.is_none());
                if has_null {
                    bail!(ValidacaoError::ValorNulo(campo.nome.clone()));
                }
            }
        }

        if let (Some(min), Some(max)) = (
            contrato.constraints.get(&campo.nome).and_then(|c| c.min),
            contrato.constraints.get(&campo.nome).and_then(|c| c.max),
        ) {
            if let Ok(ser) = col.f64() {
                for v in ser.iter().flatten() {
                    if v < min || v > max {
                        bail!(ValidacaoError::ValorForaDeRange {
                            campo: campo.nome.clone(),
                            valor: v,
                        });
                    }
                }
            }
        }
    }

    if df.height() == 0 {
        bail!("DataFrame vazio — viola constraint required");
    }

    Ok(())
}

fn main() -> Result<()> {
    println!("pipeline strict: rejeita dados que violam contrato");

    let contrato = parse_contrato("dados/esquema_vendas.json")?;
    println!(
        "contrato schema={}, v{}",
        contrato.schema.nome, contrato.schema.versao
    );

    let df_valido = DataFrame::new(
        3,
        vec![
            Column::new("id".into(), &[1i64, 2, 3]),
            Column::new("valor".into(), &[100.0, 500.0, 900.0]),
            Column::new("produto".into(), &["A", "B", "C"]),
            Column::new("regiao".into(), &["Sul", "Sudeste", "Norte"]),
        ],
    )?;

    println!("validando dados válidos...");
    validar(&df_valido, &contrato)?;
    println!("✓ dados válidos passaram no pipeline estrito");

    println!("validando dados inválidos (valor negativo)...");
    let df_invalido = DataFrame::new(
        2,
        vec![
            Column::new("id".into(), &[1i64, 2]),
            Column::new("valor".into(), &[100.0, -50.0]),
            Column::new("produto".into(), &["A", "B"]),
            Column::new("regiao".into(), &["Sul", "Sudeste"]),
        ],
    )?;

    if let Err(e) = validar(&df_invalido, &contrato) {
        println!("rejeitado: {:?}", e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrato_valido_passa() {
        let contrato = Contrato {
            schema: SchemaInfo {
                nome: "teste".into(),
                versao: 1,
                campos: vec![
                    CampoSchema {
                        nome: "id".into(),
                        tipo: "int64".into(),
                        required: true,
                    },
                    CampoSchema {
                        nome: "valor".into(),
                        tipo: "float64".into(),
                        required: true,
                    },
                ],
            },
            constraints: HashMap::new(),
        };

        let df = DataFrame::new(
            2,
            vec![
                Column::new("id".into(), &[1i64, 2]),
                Column::new("valor".into(), &[10.0, 20.0]),
            ],
        )
        .unwrap();

        assert!(validar(&df, &contrato).is_ok());
    }

    #[test]
    fn valor_fora_de_range_rejeita() {
        let mut constraints = HashMap::new();
        constraints.insert(
            "valor".into(),
            Constraint {
                unique: None,
                required: None,
                min: Some(0.0),
                max: Some(100.0),
            },
        );

        let contrato = Contrato {
            schema: SchemaInfo {
                nome: "teste".into(),
                versao: 1,
                campos: vec![
                    CampoSchema {
                        nome: "id".into(),
                        tipo: "int64".into(),
                        required: true,
                    },
                    CampoSchema {
                        nome: "valor".into(),
                        tipo: "float64".into(),
                        required: true,
                    },
                ],
            },
            constraints,
        };

        let df = DataFrame::new(
            1,
            vec![
                Column::new("id".into(), &[1i64]),
                Column::new("valor".into(), &[200.0]),
            ],
        )
        .unwrap();

        assert!(validar(&df, &contrato).is_err());
    }

    #[test]
    fn coluna_faltante_rejeita() {
        let contrato = Contrato {
            schema: SchemaInfo {
                nome: "teste".into(),
                versao: 1,
                campos: vec![
                    CampoSchema {
                        nome: "id".into(),
                        tipo: "int64".into(),
                        required: true,
                    },
                    CampoSchema {
                        nome: "valor".into(),
                        tipo: "float64".into(),
                        required: true,
                    },
                ],
            },
            constraints: HashMap::new(),
        };

        let df = DataFrame::new(1, vec![Column::new("id".into(), &[1i64])]).unwrap();
        assert!(validar(&df, &contrato).is_err());
    }
}
