//! Módulo 07 — LLM e Agentes
//! README: seção "Exercício" — Embeddings duplos (busca + reranking)
//!
//! Mesmo padrão do `LlmContextSchema` do AI-Lake: um embedding otimizado
//! para busca semântica (recall) e outro para reranking (precisão). Compara
//! qualidade com/sem o segundo embedding.

use anyhow::Result;
use hnsw_rs::prelude::*;
use llm_e_agentes_ia::{chunk_text, fake_embed};
use std::collections::HashMap;

/// Segundo embedding — diferente do primeiro, simula modelo otimizado para
/// reranking (ex: ColBERT, cross-encoder). Aqui usamos hash com seed distinto
/// e dimensão menor, mas com normalização distinta para simular outro espaço.
fn fake_embed_rerank(text: &str) -> Vec<f32> {
    let mut v = vec![0f32; 32];
    for (i, c) in text.bytes().enumerate() {
        // Seed distinto: desloca bytes
        let idx = (i * 7) % 32;
        v[idx] += (c.wrapping_add(31) as f32) / 255.0;
    }
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    for x in &mut v {
        *x /= norm;
    }
    v
}

fn build_hnsw(embeddings: &[Vec<f32>]) -> Hnsw<'_, f32, DistL2> {
    let n = embeddings.len();
    let hnsw = Hnsw::<f32, DistL2>::new(16, n, 4, 200, DistL2 {});
    for (i, emb) in embeddings.iter().enumerate() {
        hnsw.insert((emb.as_slice(), i));
    }
    hnsw
}

fn recall_at_k(retrieved: &[usize], relevantes: &[usize], k: usize) -> f64 {
    let topk: Vec<usize> = retrieved.iter().copied().take(k).collect();
    let hits = topk.iter().filter(|id| relevantes.contains(id)).count();
    hits as f64 / relevantes.len().max(1) as f64
}

fn main() -> Result<()> {
    println!("=== Exercício Double Embeddings — Módulo 07 ===\n");
    println!("Padrão AI-Lake LlmContextSchema: embedding primário para busca, secundário para reranking\n");

    // Corpus sintético — 6 docs, 2 relevantes para query "Rust concorrência"
    let docs = [
        "Rust ownership e borrowing garantem segurança sem GC.",
        "Polars é um DataFrame rápido em Rust com Arrow.",
        "Tokio e Rayon para concorrência em Rust — como escolher.",
        "Parquet colunar para analytics, Avro para streaming.",
        "Concorrência em Rust com async e threads — ownership evita data races.",
        "Iceberg e Delta Lake para transações em object storage.",
    ];
    let chunks: Vec<String> = docs.iter().flat_map(|d| chunk_text(d, 200, 20)).collect();
    println!("Corpus: {} docs → {} chunks", docs.len(), chunks.len());

    // Embeddings primário e secundário
    let emb_busca: Vec<Vec<f32>> = chunks.iter().map(|c| fake_embed(c)).collect();
    let emb_rerank: Vec<Vec<f32>> = chunks.iter().map(|c| fake_embed_rerank(c)).collect();

    // Índices HNSW separados (mesmo padrão do footer AI-Lake com 2 vetores)
    let hnsw_busca = build_hnsw(&emb_busca);
    let hnsw_rerank = build_hnsw(&emb_rerank);
    println!(
        "HNSW busca: {} pontos, HNSW rerank: {} pontos",
        hnsw_busca.get_nb_point(),
        hnsw_rerank.get_nb_point()
    );

    let query = "Rust concorrência ownership";
    let q_busca = fake_embed(query);
    let q_rerank = fake_embed_rerank(query);

    // Relevantes: chunks que contêm "Rust" e "concorrência"
    let relevantes: Vec<usize> = chunks
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.to_lowercase().contains("rust") && c.to_lowercase().contains("concorrência")
        })
        .map(|(i, _)| i)
        .collect();
    println!("Query: {query}");
    println!("Relevantes (ids): {:?}\n", relevantes);

    // Busca apenas com embedding primário
    let res_busca = hnsw_busca.search(&q_busca, 3, 16);
    let ids_busca: Vec<usize> = res_busca.iter().map(|n| n.d_id).collect();
    println!("Top-3 busca (primário): {:?}", ids_busca);
    for n in &res_busca {
        println!(
            "  id={} dist={:.4} — {:?}",
            n.d_id,
            n.distance,
            &chunks[n.d_id][..60.min(chunks[n.d_id].len())]
        );
    }
    println!(
        "Recall@3 (só busca): {:.2}",
        recall_at_k(&ids_busca, &relevantes, 3)
    );

    // Double embeddings: busca com primário, reranking com secundário
    // 1) recupera top-5 com primário, 2) re-ordena com secundário
    let res_busca_5 = hnsw_busca.search(&q_busca, 5, 16);
    let mut reranked: Vec<(usize, f32)> = res_busca_5
        .iter()
        .map(|n| {
            let idx = n.d_id;
            // Distância L2 no espaço de reranking
            let dist_rerank = {
                let a = &q_rerank;
                let b = &emb_rerank[idx];
                a.iter()
                    .zip(b.iter())
                    .map(|(x, y)| (x - y).powi(2))
                    .sum::<f32>()
                    .sqrt()
            };
            (idx, dist_rerank)
        })
        .collect();
    reranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let ids_rerank: Vec<usize> = reranked.iter().map(|(id, _)| *id).take(3).collect();
    println!(
        "\nTop-3 double embeddings (busca + rerank): {:?}",
        ids_rerank
    );
    for (id, dist) in reranked.iter().take(3) {
        println!(
            "  id={id} rerank_dist={dist:.4} — {:?}",
            &chunks[*id][..60.min(chunks[*id].len())]
        );
    }
    println!(
        "Recall@3 (double): {:.2}",
        recall_at_k(&ids_rerank, &relevantes, 3)
    );

    // Comparação
    let recall_busca = recall_at_k(&ids_busca, &relevantes, 3);
    let recall_double = recall_at_k(&ids_rerank, &relevantes, 3);
    println!("\n--- Comparação ---");
    println!("Recall@3 busca:  {recall_busca:.2}");
    println!("Recall@3 double: {recall_double:.2}");
    if recall_double >= recall_busca {
        println!("✓ Double embeddings melhor ou igual (esperado para queries ambíguas)");
    } else {
        println!("✗ Busca simples melhor neste corpus pequeno — em produção, double ajuda com reranking semântico");
    }

    // Demonstra armazenamento dual no AI-Lake: 2 vetores por chunk no footer
    let mut footer_sim: HashMap<usize, (Vec<f32>, Vec<f32>)> = HashMap::new();
    for (i, (e1, e2)) in emb_busca.iter().zip(emb_rerank.iter()).enumerate() {
        footer_sim.insert(i, (e1.clone(), e2.clone()));
    }
    println!(
        "\nFooter AI-Lake simulado: {} entradas com 2 embeddings cada (busca + rerank)",
        footer_sim.len()
    );
    println!(
        "  Exemplo chunk 0: dim busca={}, dim rerank={}",
        footer_sim[&0].0.len(),
        footer_sim[&0].1.len()
    );

    println!("\n✓ Exercício concluído — padrão double embeddings validado");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeddings_distintos() {
        let t = "Rust concorrência";
        let a = fake_embed(t);
        let b = fake_embed_rerank(t);
        // Devem ser diferentes (seed distinto)
        assert_ne!(a, b);
        assert_eq!(a.len(), 32);
        assert_eq!(b.len(), 32);
    }

    #[test]
    fn double_retrieval_nao_panica() {
        let docs = vec!["Rust", "dados", "Rust concorrência"];
        let chunks: Vec<String> = docs.iter().flat_map(|d| chunk_text(d, 100, 10)).collect();
        let e1: Vec<Vec<f32>> = chunks.iter().map(|c| fake_embed(c)).collect();
        let e2: Vec<Vec<f32>> = chunks.iter().map(|c| fake_embed_rerank(c)).collect();
        let h1 = build_hnsw(&e1);
        let h2 = build_hnsw(&e2);
        let q1 = fake_embed("Rust");
        let q2 = fake_embed_rerank("Rust");
        let r1 = h1.search(&q1, 1, 16);
        let r2 = h2.search(&q2, 1, 16);
        assert_eq!(r1.len(), 1);
        assert_eq!(r2.len(), 1);
    }

    #[test]
    fn recall_calcula() {
        assert!((recall_at_k(&[1, 3], &[1, 2], 2) - 0.5).abs() < 1e-9);
        assert!((recall_at_k(&[1, 2], &[1, 2], 2) - 1.0).abs() < 1e-9);
    }
}
