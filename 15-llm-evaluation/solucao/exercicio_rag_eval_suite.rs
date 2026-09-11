//! Suite de avaliação RAG com gate de qualidade (exercício).
//!
//! Roda recall/faithfulness/groundedness em N casos, gera `relatorio.json`
//! e retorna erro se faithfulness média < limiar — pronto para gate de CI.

use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;

/// Reimplementação local das métricas (suite deve ser autocontida).
fn tokens(texto: &str) -> HashSet<String> {
    texto
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

#[derive(Debug, Clone)]
struct Caso {
    pergunta: &'static str,
    relevantes: Vec<usize>,
    recuperados: Vec<usize>,
    contextos: Vec<&'static str>,
    resposta: &'static str,
}

fn recall_at_k(c: &Caso, k: usize) -> f64 {
    if c.relevantes.is_empty() {
        return 1.0;
    }
    let top: HashSet<usize> = c.recuperados.iter().take(k).copied().collect();
    let rel: HashSet<usize> = c.relevantes.iter().copied().collect();
    top.intersection(&rel).count() as f64 / rel.len() as f64
}

fn faithfulness(c: &Caso) -> f64 {
    let resp = tokens(c.resposta);
    if resp.is_empty() {
        return 0.0;
    }
    let ctx: HashSet<String> = c.contextos.iter().flat_map(|t| tokens(t)).collect();
    resp.intersection(&ctx).count() as f64 / resp.len() as f64
}

fn groundedness(c: &Caso) -> f64 {
    let ctx: HashSet<String> = c.contextos.iter().flat_map(|t| tokens(t)).collect();
    if ctx.is_empty() {
        return 0.0;
    }
    let resp = tokens(c.resposta);
    resp.intersection(&ctx).count() as f64 / ctx.len() as f64
}

#[derive(Debug, Serialize)]
struct LinhaRelatorio {
    pergunta: String,
    recall_at_3: f64,
    faithfulness: f64,
    groundedness: f64,
    passou: bool,
}

/// Limiar de faithfulness por caso e na média (gate de CI).
const LIMIAR: f64 = 0.6;

fn casos() -> Vec<Caso> {
    vec![
        Caso {
            pergunta: "total sudeste?",
            relevantes: vec![0],
            recuperados: vec![0, 1],
            contextos: vec!["sudeste vendeu 11700"],
            resposta: "sudeste vendeu 11700",
        },
        Caso {
            pergunta: "zona franca onde?",
            relevantes: vec![1],
            recuperados: vec![1],
            contextos: vec!["norte tem zona franca"],
            resposta: "zona franca fica no norte",
        },
        Caso {
            pergunta: "produto mais caro?",
            relevantes: vec![2],
            recuperados: vec![2, 0],
            contextos: vec!["notebook custa 5500"],
            resposta: "notebook custa 5500 com frete gratis para sempre",
        },
        Caso {
            pergunta: "regiao com turismo?",
            relevantes: vec![3],
            recuperados: vec![5, 3],
            contextos: vec!["nordeste tem turismo e energia solar"],
            resposta: "nordeste tem turismo",
        },
    ]
}

fn main() -> Result<()> {
    let dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("dados/saida"));
    std::fs::create_dir_all(&dir).context("criando dir de saída")?;

    let mut linhas = Vec::new();
    for c in casos() {
        let r = recall_at_k(&c, 3);
        let f = faithfulness(&c);
        let g = groundedness(&c);
        let passou = f >= LIMIAR;
        println!(
            "{:<22} recall@3={r:.2} faithfulness={f:.2} groundedness={g:.2} {}",
            c.pergunta,
            if passou { "PASS" } else { "FAIL" }
        );
        linhas.push(LinhaRelatorio {
            pergunta: c.pergunta.to_string(),
            recall_at_3: r,
            faithfulness: f,
            groundedness: g,
            passou,
        });
    }

    let media_f: f64 = linhas.iter().map(|l| l.faithfulness).sum::<f64>() / linhas.len() as f64;
    let relatorio = serde_json::json!({
        "limiar_faithfulness": LIMIAR,
        "faithfulness_media": media_f,
        "casos": linhas,
    });
    let path = dir.join("relatorio_rag_eval.json");
    std::fs::write(&path, serde_json::to_string_pretty(&relatorio)?)?;
    println!("\nfaithfulness média: {media_f:.2} → {}", path.display());

    if media_f < LIMIAR {
        bail!("gate de qualidade: faithfulness média {media_f:.2} < {LIMIAR}");
    }
    println!("gate de qualidade: PASS");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_media_acima_do_limiar() {
        let casos = casos();
        let media: f64 = casos.iter().map(faithfulness).sum::<f64>() / casos.len() as f64;
        assert!(media >= LIMIAR, "media {media:.2} deveria passar no gate");
    }

    #[test]
    fn caso_alucinado_falha_individual() {
        let c = &casos()[2];
        assert!(faithfulness(c) < LIMIAR);
    }

    #[test]
    fn recall_cobre_recuperacao() {
        for c in casos() {
            assert!(recall_at_k(&c, 3) > 0.0);
        }
    }
}
