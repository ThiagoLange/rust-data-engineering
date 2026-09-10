//! Projeto 1 — Mini Lakehouse com busca vetorial
//! README 11 + `projeto-1-mini-lakehouse/ARCHITECTURE.md`.
//!
//! Ingestão → Parquet particionado com footer HNSW → embeddings → HNSW →
//! consulta híbrida (SQL Polars + busca vetorial).

use anyhow::Result;
use hnsw_rs::prelude::*;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::metadata::KeyValue;
use parquet::file::properties::WriterProperties;
use parquet::file::reader::FileReader;
use polars::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct Produto {
    id: u64,
    nome: String,
    descricao: String,
    categoria: String,
    preco: f64,
}

fn gerar_produtos() -> Vec<Produto> {
    vec![
        Produto {
            id: 1,
            nome: "Notebook Pro".into(),
            descricao: "notebook potente para desenvolvimento rust e dados".into(),
            categoria: "eletronicos".into(),
            preco: 5500.0,
        },
        Produto {
            id: 2,
            nome: "Mouse sem fio".into(),
            descricao: "mouse ergonômico para escritório".into(),
            categoria: "eletronicos".into(),
            preco: 120.0,
        },
        Produto {
            id: 3,
            nome: "Camiseta algodão".into(),
            descricao: "camiseta confortável de algodão".into(),
            categoria: "roupas".into(),
            preco: 80.0,
        },
        Produto {
            id: 4,
            nome: "Livro Rust".into(),
            descricao: "livro sobre ownership concorrência e async em rust".into(),
            categoria: "livros".into(),
            preco: 150.0,
        },
        Produto {
            id: 5,
            nome: "Monitor 4k".into(),
            descricao: "monitor para dados e programação".into(),
            categoria: "eletronicos".into(),
            preco: 2200.0,
        },
        Produto {
            id: 6,
            nome: "Livro Parquet".into(),
            descricao: "livro sobre parquet delta lake e iceberg".into(),
            categoria: "livros".into(),
            preco: 130.0,
        },
    ]
}

/// Embedding determinístico 32d (mesmo padrão do Módulo 7).
fn fake_embed(text: &str) -> Vec<f32> {
    let mut v = vec![0f32; 32];
    for (i, c) in text.bytes().enumerate() {
        v[i % 32] += c as f32 / 255.0;
    }
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    for x in &mut v {
        *x /= norm;
    }
    v
}

fn produtos_para_df(produtos: &[Produto]) -> Result<DataFrame> {
    let n = produtos.len();
    let ids: Vec<u64> = produtos.iter().map(|p| p.id).collect();
    let nomes: Vec<String> = produtos.iter().map(|p| p.nome.clone()).collect();
    let descs: Vec<String> = produtos.iter().map(|p| p.descricao.clone()).collect();
    let cats: Vec<String> = produtos.iter().map(|p| p.categoria.clone()).collect();
    let precos: Vec<f64> = produtos.iter().map(|p| p.preco).collect();
    DataFrame::new(
        n,
        vec![
            Column::new("id".into(), ids),
            Column::new("nome".into(), nomes),
            Column::new("descricao".into(), descs),
            Column::new("categoria".into(), cats),
            Column::new("preco".into(), precos),
        ],
    )
    .map_err(|e| anyhow::anyhow!("df: {e}"))
}

/// Escreve Parquet particionado por `categoria=` com footer HNSW simulado.
fn escrever_lakehouse(df: &DataFrame, embeddings: &[Vec<f32>], out_dir: &Path) -> Result<()> {
    // Converte DF para RecordBatch via Arrow (Polars → Arrow)
    let df_clone = df.clone();
    // Escreve um Parquet por categoria (particionamento manual estilo Hive)
    let cats: Vec<String> = df
        .column("categoria")
        .map_err(|e| anyhow::anyhow!("categoria: {e}"))?
        .str()
        .map_err(|e| anyhow::anyhow!("str: {e}"))?
        .iter()
        .map(|o| o.unwrap_or("").to_string())
        .collect();
    let mut por_cat: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, c) in cats.iter().enumerate() {
        por_cat.entry(c.clone()).or_default().push(i);
    }

    for (cat, idxs) in por_cat {
        let sub = df_clone.slice(idxs[0] as i64, idxs.len());
        let batch = df_to_batch(&sub)?;
        let dir = out_dir.join(format!("categoria={cat}"));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("part-0.parquet");
        let kv = vec![
            KeyValue::new("ailake.format_version".to_string(), "1.0".to_string()),
            KeyValue::new("ailake.index_type".to_string(), "hnsw".to_string()),
            KeyValue::new("ailake.vector_column".to_string(), "embedding".to_string()),
            KeyValue::new("ailake.hnsw_m".to_string(), "16".to_string()),
        ];
        let props = WriterProperties::builder()
            .set_key_value_metadata(Some(kv))
            .build();
        let file = File::create(&path)?;
        let mut w = ArrowWriter::try_new(file, batch.schema(), Some(props))?;
        w.write(&batch)?;
        w.close()?;
        println!(
            "  partição categoria={cat}: {} linhas → {}",
            idxs.len(),
            path.display()
        );
    }
    // Amostra: mostra que footer sobrevive
    let sample = out_dir.join("categoria=eletronicos/part-0.parquet");
    if sample.exists() {
        let f = File::open(&sample)?;
        let r = parquet::file::reader::SerializedFileReader::new(f)?;
        let kv = r
            .metadata()
            .file_metadata()
            .key_value_metadata()
            .cloned()
            .unwrap_or_default();
        println!(
            "  footer AI-Lake: {} chaves (ailake.index_type={})",
            kv.len(),
            kv.iter()
                .find(|k| k.key == "ailake.index_type")
                .and_then(|k| k.value.clone())
                .unwrap_or_default()
        );
    }
    let _ = embeddings;
    Ok(())
}

fn df_to_batch(df: &DataFrame) -> Result<arrow::record_batch::RecordBatch> {
    use arrow::array::{Float64Array, StringArray, UInt64Array};
    use arrow_schema::{DataType, Field, Schema};
    use std::sync::Arc;
    let n = df.height();
    let ids: UInt64Array = (0..n)
        .map(|i| df.column("id").ok()?.u64().ok()?.get(i))
        .collect();
    let nomes: StringArray = (0..n)
        .map(|i| {
            df.column("nome")
                .ok()?
                .str()
                .ok()?
                .get(i)
                .map(|s| s.to_string())
        })
        .collect();
    let cats: StringArray = (0..n)
        .map(|i| {
            df.column("categoria")
                .ok()?
                .str()
                .ok()?
                .get(i)
                .map(|s| s.to_string())
        })
        .collect();
    let precos: Float64Array = (0..n)
        .map(|i| df.column("preco").ok()?.f64().ok()?.get(i))
        .collect();
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::UInt64, false),
        Field::new("nome", DataType::Utf8, false),
        Field::new("categoria", DataType::Utf8, false),
        Field::new("preco", DataType::Float64, false),
    ]));
    arrow::record_batch::RecordBatch::try_new(
        schema,
        vec![
            Arc::new(ids),
            Arc::new(nomes),
            Arc::new(cats),
            Arc::new(precos),
        ],
    )
    .map_err(|e| anyhow::anyhow!("batch: {e}"))
}

fn build_hnsw(embeddings: &[Vec<f32>]) -> Hnsw<'_, f32, DistL2> {
    let n = embeddings.len();
    let hnsw = Hnsw::<f32, DistL2>::new(16, n, 4, 200, DistL2 {});
    for (i, e) in embeddings.iter().enumerate() {
        hnsw.insert((e.as_slice(), i));
    }
    hnsw
}

/// Consulta híbrida: filtro SQL (categoria + preco) ∩ busca vetorial top-k,
/// fusão por interseção com reordenação vetorial.
fn consulta_hibrida(
    produtos: &[Produto],
    _embeddings: &[Vec<f32>],
    hnsw: &Hnsw<f32, DistL2>,
    query: &str,
    categoria: Option<&str>,
    preco_max: f64,
) -> Vec<(usize, f32)> {
    let q = fake_embed(query);
    let res = hnsw.search(&q, 4, 16);
    // Fusão: mantém candidatos que passam no filtro SQL, ordenados por distância
    let mut out = Vec::new();
    for n in res {
        let p = &produtos[n.d_id];
        let passa_sql = categoria.map(|c| p.categoria == c).unwrap_or(true) && p.preco <= preco_max;
        if passa_sql {
            out.push((n.d_id, n.distance));
        }
    }
    out.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    out
}

fn main() -> Result<()> {
    println!("=== Projeto 1 — Mini Lakehouse vetorial ===\n");
    let out_dir = PathBuf::from("dados/saida/lakehouse");
    let _ = std::fs::remove_dir_all(&out_dir);
    std::fs::create_dir_all(&out_dir)?;

    // 1. Ingestão
    let produtos = gerar_produtos();
    let df = produtos_para_df(&produtos)?;
    println!(
        "Ingestão: {} produtos\n{}",
        produtos.len(),
        df.head(Some(3))
    );

    // 2. Embeddings
    let embeddings: Vec<Vec<f32>> = produtos
        .iter()
        .map(|p| fake_embed(&format!("{} {}", p.nome, p.descricao)))
        .collect();
    println!("\nEmbeddings: {} vetores dim=32", embeddings.len());

    // 3. Lakehouse (Parquet + footer)
    println!("\nEscrevendo lakehouse particionado:");
    escrever_lakehouse(&df, &embeddings, &out_dir)?;

    // 4. HNSW
    let hnsw = build_hnsw(&embeddings);
    println!("\nHNSW: {} pontos", hnsw.get_nb_point());

    // 5. Consulta híbrida
    let query = "livro sobre rust e dados";
    println!("\nConsulta híbrida: query={query:?} + SQL(categoria=livros, preco<=200)");
    let res = consulta_hibrida(&produtos, &embeddings, &hnsw, query, Some("livros"), 200.0);
    for (id, dist) in &res {
        let p = &produtos[*id];
        println!(
            "  id={} dist={dist:.4} {} ({}, R${:.2})",
            p.id, p.nome, p.categoria, p.preco
        );
    }

    // Leitura de volta do Parquet (valida round-trip)
    let sample = out_dir.join("categoria=livros/part-0.parquet");
    let f = File::open(&sample)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(f)?.build()?;
    let batches: Vec<_> = reader.collect::<Result<Vec<_>, _>>()?;
    let total: usize = batches.iter().map(|b| b.num_rows()).sum();
    println!("\nRound-trip Parquet livros: {total} linhas");

    println!("\n✓ Mini lakehouse OK (Parquet + footer HNSW + SQL + vetorial)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn lakehouse_round_trip() {
        let dir = TempDir::new().unwrap();
        let produtos = gerar_produtos();
        let df = produtos_para_df(&produtos).expect("ok");
        let embs: Vec<Vec<f32>> = produtos.iter().map(|p| fake_embed(&p.nome)).collect();
        escrever_lakehouse(&df, &embs, dir.path()).expect("ok");
        assert!(dir.path().join("categoria=livros/part-0.parquet").exists());
    }

    #[test]
    fn hibrida_retorna_livros() {
        let produtos = gerar_produtos();
        let embs: Vec<Vec<f32>> = produtos
            .iter()
            .map(|p| fake_embed(&format!("{} {}", p.nome, p.descricao)))
            .collect();
        let hnsw = build_hnsw(&embs);
        let res = consulta_hibrida(&produtos, &embs, &hnsw, "livro rust", Some("livros"), 200.0);
        assert!(!res.is_empty());
        for (id, _) in res {
            assert_eq!(produtos[id].categoria, "livros");
        }
    }

    #[test]
    fn footer_tem_hnsw() {
        let dir = TempDir::new().unwrap();
        let produtos = gerar_produtos();
        let df = produtos_para_df(&produtos).expect("ok");
        let embs: Vec<Vec<f32>> = produtos.iter().map(|p| fake_embed(&p.nome)).collect();
        escrever_lakehouse(&df, &embs, dir.path()).expect("ok");
        let f = File::open(dir.path().join("categoria=eletronicos/part-0.parquet")).expect("ok");
        let r = parquet::file::reader::SerializedFileReader::new(f).expect("ok");
        let kv = r
            .metadata()
            .file_metadata()
            .key_value_metadata()
            .cloned()
            .unwrap_or_default();
        assert!(kv.iter().any(|k| k.key == "ailake.index_type"));
    }
}
