//! Módulo 07 — LLM e Agentes
//! README: seção "Exemplo prático 1: Pipeline RAG completo"
//!
//! Ingestão Markdown → chunking → embeddings (async-openai ou fake) → HNSW
//! (hnsw_rs, mesmo padrão AI-Lake) → busca → prompt + LLM. Sem API key, roda
//! 100% offline com embeddings sintéticos e LLM simulado.

use anyhow::{Context, Result};
use clap::Parser;
use hnsw_rs::prelude::*;
use llm_e_agentes_ia::{call_llm, chunk_text, embed};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "rag_pipeline")]
struct Args {
    /// Diretório com documentos Markdown
    #[arg(long, default_value = "documentos")]
    docs: PathBuf,

    /// Pergunta para busca
    #[arg(long, default_value = "O que é Rust e como ele lida com concorrência?")]
    query: String,

    /// Número de chunks a recuperar
    #[arg(long, default_value_t = 3)]
    top_k: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    println!("=== RAG Pipeline — Módulo 07 ===\n");
    println!("Docs: {}", args.docs.display());
    println!("Query: {}", args.query);
    if std::env::var("OPENAI_API_KEY").is_err() && std::env::var("ANTHROPIC_API_KEY").is_err() {
        println!("(sem API key — usando embeddings e LLM simulados. Veja .env.example)");
    } else {
        println!("(API key detectada — tentará chamadas reais, fallback para simulado se falhar)");
    }
    // AVISO custo
    println!("Aviso: chamadas a APIs pagas limitadas a 512 tokens por padrão.\n");

    // 1. Ingestão + chunking
    let chunks = ingest(&args.docs).await?;
    println!(
        "Ingestão: {} chunks de {} arquivos",
        chunks.len(),
        args.docs.display()
    );
    for (i, c) in chunks.iter().take(2).enumerate() {
        println!(
            "  chunk {i}: {} chars — {:?}",
            c.len(),
            &c[..60.min(c.len())]
        );
    }

    // 2. Embeddings
    println!("\nGerando embeddings ({} chunks)...", chunks.len());
    let mut embeddings = Vec::with_capacity(chunks.len());
    for chunk in &chunks {
        let v = embed(chunk).await?;
        embeddings.push(v);
    }
    println!(
        "Embeddings: {} vetores dim={}",
        embeddings.len(),
        embeddings[0].len()
    );

    // 3. HNSW — mesmo padrão do AI-Lake footer
    println!("\nIndexando com HNSW (hnsw_rs)...");
    let hnsw = build_hnsw(&embeddings)?;
    println!("HNSW: {} pontos indexados", hnsw.get_nb_point());

    // 4. Busca
    let query_emb = embed(&args.query).await?;
    let neighbours = hnsw.search(&query_emb, args.top_k, 32);
    println!("\nBusca top-{} para query:", args.top_k);
    for n in &neighbours {
        let idx = n.d_id;
        let dist = n.distance;
        // Similaridade = 1 - distância (para DistDot, menor distância = mais similar)
        println!(
            "  d_id={idx} dist={dist:.4} — {:?}",
            &chunks[idx][..80.min(chunks[idx].len())]
        );
    }

    // 5. Reranking simples (opcional) — re-ordena por BM25-like lexical overlap
    let reranked = rerank(&args.query, &chunks, &neighbours);
    println!("\nReranking lexical (top {}):", reranked.len());
    for (idx, score) in &reranked {
        println!("  d_id={idx} score={score:.3}");
    }

    // 6. Prompt + LLM
    let contexto = reranked
        .iter()
        .map(|(idx, _)| chunks[*idx].as_str())
        .collect::<Vec<_>>()
        .join("\n---\n");
    let prompt = format!(
        "Use o contexto abaixo para responder à pergunta.\n\nContexto:\n{contexto}\n\nPergunta: {}\nResposta:",
        args.query
    );
    println!(
        "\nPrompt ({} chars):\n{}\n",
        prompt.len(),
        &prompt[..500.min(prompt.len())]
    );

    let resposta = call_llm(&prompt).await?;
    println!("=== Resposta LLM ===\n{resposta}\n");
    println!("✓ RAG pipeline concluído (HNSW + reranking + LLM)");

    Ok(())
}

async fn ingest(dir: &Path) -> Result<Vec<String>> {
    let mut chunks = Vec::new();
    let mut rd = tokio::fs::read_dir(dir)
        .await
        .with_context(|| format!("lendo {}", dir.display()))?;
    while let Some(entry) = rd.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let text = tokio::fs::read_to_string(&path).await?;
            let cs = chunk_text(&text, 500, 50);
            for c in cs {
                chunks.push(c);
            }
        }
    }
    if chunks.is_empty() {
        anyhow::bail!("nenhum .md encontrado em {}", dir.display());
    }
    Ok(chunks)
}

fn build_hnsw(embeddings: &[Vec<f32>]) -> Result<Hnsw<'_, f32, DistDot>> {
    let nb_elem = embeddings.len();
    let max_nb_conn = 16;
    let ef_c = 200;
    let nb_layer = 16.min((nb_elem as f32).ln().trunc() as usize).max(1);
    let hnsw = Hnsw::<f32, DistDot>::new(max_nb_conn, nb_elem, nb_layer, ef_c, DistDot {});
    for (i, emb) in embeddings.iter().enumerate() {
        hnsw.insert((emb.as_slice(), i));
    }
    Ok(hnsw)
}

fn rerank(query: &str, chunks: &[String], neighbours: &[Neighbour]) -> Vec<(usize, f32)> {
    // Score lexical simples: overlap de tokens
    let q_tokens: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    let mut scored: Vec<(usize, f32)> = neighbours
        .iter()
        .map(|n| {
            let idx = n.d_id;
            let text = &chunks[idx].to_lowercase();
            let overlap = q_tokens.iter().filter(|t| text.contains(*t)).count() as f32
                / q_tokens.len() as f32;
            // Combina distância vetorial (1-dist) com lexical (0.3 peso)
            let vec_score = 1.0 - n.distance.clamp(0.0, 1.0);
            let final_score = 0.7 * vec_score + 0.3 * overlap;
            (idx, final_score)
        })
        .collect();
    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use llm_e_agentes_ia::fake_embed;

    #[test]
    fn chunking_basico() {
        let text = "a".repeat(600);
        let cs = llm_e_agentes_ia::chunk_text(&text, 500, 50);
        assert!(cs.len() >= 2);
    }

    #[test]
    fn hnsw_busca() {
        let embs = vec![
            fake_embed("rust"),
            fake_embed("dados"),
            fake_embed("rust concorrência"),
        ];
        let hnsw = build_hnsw(&embs).expect("ok");
        let q = fake_embed("rust");
        let res = hnsw.search(&q, 1, 16);
        assert_eq!(res.len(), 1);
    }

    #[tokio::test]
    async fn ingest_lê_documentos() {
        let chunks = ingest(Path::new("documentos")).await.expect("ok");
        assert!(!chunks.is_empty());
    }
}
