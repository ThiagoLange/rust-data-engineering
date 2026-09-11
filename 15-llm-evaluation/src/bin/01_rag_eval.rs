//! Métricas de avaliação de RAG: recall@k, faithfulness e groundedness.
//!
//! Seção correspondente no README: Exemplos → `01_rag_eval`.
//!
//! As métricas usam overlap de tokens normalizados (minúsculas, sem pontuação):
//! didático e sem custo de API. Em produção, troque o comparador por um
//! juiz-LLM mantendo as mesmas assinaturas.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Um caso do golden set: o que deveria ser recuperado e respondido.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasoAvaliacao {
    pub pergunta: String,
    /// IDs dos documentos relevantes (verdade).
    pub docs_relevantes: Vec<usize>,
    /// IDs retornados pelo retriever, em ordem de score.
    pub docs_recuperados: Vec<usize>,
    /// Trechos de contexto entregues ao gerador.
    pub contextos: Vec<String>,
    /// Resposta gerada pelo pipeline.
    pub resposta: String,
}

/// Tokeniza: minúsculas, mantém alfanuméricos, divide por whitespace.
pub fn tokens(texto: &str) -> HashSet<String> {
    texto
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

/// recall@k = |relevantes ∩ recuperados[:k]| / |relevantes|.
pub fn recall_at_k(caso: &CasoAvaliacao, k: usize) -> f64 {
    if caso.docs_relevantes.is_empty() {
        return 1.0;
    }
    let top: HashSet<usize> = caso.docs_recuperados.iter().take(k).copied().collect();
    let relevantes: HashSet<usize> = caso.docs_relevantes.iter().copied().collect();
    top.intersection(&relevantes).count() as f64 / relevantes.len() as f64
}

/// Faithfulness: fração dos tokens da resposta sustentada pelos contextos.
pub fn faithfulness(caso: &CasoAvaliacao) -> f64 {
    let resp = tokens(&caso.resposta);
    if resp.is_empty() {
        return 0.0;
    }
    let ctx: HashSet<String> = caso.contextos.iter().flat_map(|c| tokens(c)).collect();
    resp.intersection(&ctx).count() as f64 / resp.len() as f64
}

/// Groundedness: fração dos tokens dos contextos coberta pela resposta.
/// Mede se a resposta aproveita o contexto (complemento do faithfulness).
pub fn groundedness(caso: &CasoAvaliacao) -> f64 {
    let ctx: HashSet<String> = caso.contextos.iter().flat_map(|c| tokens(c)).collect();
    if ctx.is_empty() {
        return 0.0;
    }
    let resp = tokens(&caso.resposta);
    resp.intersection(&ctx).count() as f64 / ctx.len() as f64
}

fn golden_set() -> Vec<CasoAvaliacao> {
    vec![
        CasoAvaliacao {
            pergunta: "Qual o total de vendas no Sudeste?".to_string(),
            docs_relevantes: vec![0, 2],
            docs_recuperados: vec![0, 2, 5],
            contextos: vec![
                "Sudeste vendeu 11700 em 40 transacoes".to_string(),
                "Sul vendeu 11720 em 40 transacoes".to_string(),
            ],
            resposta: "O Sudeste vendeu 11700 em 40 transacoes".to_string(),
        },
        CasoAvaliacao {
            pergunta: "Onde fica a zona franca?".to_string(),
            docs_relevantes: vec![1],
            docs_recuperados: vec![3, 1, 4],
            contextos: vec!["Norte tem zona franca e mineracao".to_string()],
            resposta: "A zona franca fica no Norte com mineracao".to_string(),
        },
        CasoAvaliacao {
            pergunta: "Qual produto mais caro?".to_string(),
            docs_relevantes: vec![4],
            docs_recuperados: vec![4],
            contextos: vec!["Notebook Pro custa 5500 reais".to_string()],
            // Alucinação: preço não está no contexto.
            resposta: "O Notebook Pro custa 5500 reais e tem garantia vitalicia".to_string(),
        },
    ]
}

fn main() -> Result<()> {
    println!("=== Avaliação RAG (golden set sintético) ===\n");
    for (i, caso) in golden_set().iter().enumerate() {
        let r1 = recall_at_k(caso, 1);
        let r3 = recall_at_k(caso, 3);
        let f = faithfulness(caso);
        let g = groundedness(caso);
        println!("caso {i}: {}", caso.pergunta);
        println!("  recall@1={r1:.2} recall@3={r3:.2} faithfulness={f:.2} groundedness={g:.2}");
    }
    println!("\nNota: caso 2 tem faithfulness < 1.0 (alucinação: 'garantia vitalicia').");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caso_base() -> CasoAvaliacao {
        CasoAvaliacao {
            pergunta: "p".to_string(),
            docs_relevantes: vec![0, 1],
            docs_recuperados: vec![0, 2, 1],
            contextos: vec!["gato preto".to_string()],
            resposta: "gato preto".to_string(),
        }
    }

    #[test]
    fn recall_perfeito_em_3() {
        assert_eq!(recall_at_k(&caso_base(), 3), 1.0);
    }

    #[test]
    fn recall_parcial_em_1() {
        assert_eq!(recall_at_k(&caso_base(), 1), 0.5);
    }

    #[test]
    fn faithfulness_total_quando_resposta_no_contexto() {
        assert_eq!(faithfulness(&caso_base()), 1.0);
    }

    #[test]
    fn faithfulness_zero_sem_contexto() {
        let mut c = caso_base();
        c.contextos = vec![];
        assert_eq!(faithfulness(&c), 0.0);
    }

    #[test]
    fn alucinacao_reduz_faithfulness() {
        let mut c = caso_base();
        c.resposta = "gato preto com asas douradas".to_string();
        let f = faithfulness(&c);
        assert!(f < 1.0 && f > 0.0);
    }
}
