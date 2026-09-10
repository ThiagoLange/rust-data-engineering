//! Módulo 03 — Armazenamento e Formatos
//! README: seção "AI Lake" — formato Lakehouse vetorial
//!
//! AI Lake combina Parquet colunar + footer HNSW vetorial + metadata Iceberg/Delta.
//! Este demo mostra o princípio: escreve Parquet com `key_value_metadata`
//! simulando um índice vetorial no footer, lê de volta e valida que o
//! metadata sobrevive ao round-trip — mesmo padrão usado no AI-Lake real.

use anyhow::{Context, Result};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::metadata::KeyValue;
use parquet::file::properties::WriterProperties;
use parquet::file::reader::FileReader;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use armazenamento_formatos::csv_utils::{ler_csv, schema_vendas};

/// Metadata que simula um índice HNSW no footer Parquet.
/// No AI-Lake real, aqui estariam: `hnsw_dist: cosine`, `hnsw_m: 16`,
/// `vector_column: embedding`, offsets do índice, etc.
fn ailake_metadata() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("ailake.format_version".to_string(), "1.0".to_string());
    m.insert("ailake.vector_column".to_string(), "embedding".to_string());
    m.insert("ailake.index_type".to_string(), "hnsw".to_string());
    m.insert("ailake.hnsw_m".to_string(), "16".to_string());
    m.insert("ailake.hnsw_ef_construction".to_string(), "200".to_string());
    m.insert("ailake.distance".to_string(), "cosine".to_string());
    // Em produção, o footer HNSW seria serializado aqui (binário + offsets)
    m.insert(
        "ailake.hnsw_footer_offset".to_string(),
        "simulado:0-4096".to_string(),
    );
    m
}

fn escrever_parquet_com_footer(caminho: &Path, batches: &[RecordBatch]) -> Result<()> {
    if batches.is_empty() {
        anyhow::bail!("nenhum batch");
    }
    let metadata = ailake_metadata();
    let kv: Vec<KeyValue> = metadata
        .into_iter()
        .map(|(k, v)| KeyValue::new(k, v))
        .collect();
    let props = WriterProperties::builder()
        .set_key_value_metadata(Some(kv))
        .build();
    let file = File::create(caminho).with_context(|| format!("criando {}", caminho.display()))?;
    let mut writer =
        ArrowWriter::try_new(file, batches[0].schema(), Some(props)).context("criando writer")?;
    for batch in batches {
        writer.write(batch).context("escrevendo batch")?;
    }
    writer.close().context("fechando")?;
    Ok(())
}

fn ler_footer(caminho: &Path) -> Result<Option<HashMap<String, String>>> {
    // Usa SerializedFileReader para acessar o footer key_value_metadata
    let file = File::open(caminho).with_context(|| format!("abrindo {}", caminho.display()))?;
    let reader =
        parquet::file::reader::SerializedFileReader::new(file).context("SerializedFileReader")?;
    let kv_vec = reader
        .metadata()
        .file_metadata()
        .key_value_metadata()
        .cloned();
    let kv: Option<HashMap<String, String>> = kv_vec.map(|vec| {
        vec.into_iter()
            .filter_map(|kv| kv.value.map(|v| (kv.key, v)))
            .collect()
    });

    // Valida que os dados ainda são legíveis via Arrow reader
    let file2 = File::open(caminho).with_context(|| format!("abrindo {}", caminho.display()))?;
    let reader2 = ParquetRecordBatchReaderBuilder::try_new(file2)
        .context("reader builder")?
        .build()
        .context("build reader")?;
    let batches: Vec<RecordBatch> = reader2
        .collect::<Result<Vec<_>, _>>()
        .context("lendo batches")?;
    let total: usize = batches.iter().map(|b| b.num_rows()).sum();
    println!("  Parquet AI Lake: {total} linhas lidas, footer validado");
    Ok(kv)
}

fn main() -> Result<()> {
    let entrada = Path::new("dados/vendas.csv");
    let saida = Path::new("dados/saida/ailake/vendas_ailake.parquet");

    let saida_parent = saida
        .parent()
        .ok_or_else(|| anyhow::anyhow!("caminho sem parent: {}", saida.display()))?;
    std::fs::create_dir_all(saida_parent).context("criando diretório")?;

    let schema = schema_vendas();
    let batches = ler_csv(entrada, schema.clone())?;
    // Para demo, concatenamos em um único batch (AI Lake grava por row-group)
    let batch_unico = {
        let schema_ref: Arc<arrow_schema::Schema> = schema.clone();
        arrow::compute::concat_batches(&schema_ref, batches.iter()).context("concat")?
    };
    let _ = schema; // evita warning
    println!(
        "Escrevendo Parquet AI Lake (Parquet + footer HNSW simulado) em {}",
        saida.display()
    );
    escrever_parquet_com_footer(saida, &[batch_unico])?;

    // Leitura e validação do footer
    let kv = ler_footer(saida)?.unwrap_or_default();
    println!("\nFooter AI Lake (key_value_metadata):");
    for (k, v) in kv.iter() {
        println!("  {k} = {v}");
    }
    anyhow::ensure!(
        kv.contains_key("ailake.index_type"),
        "footer deve conter ailake.index_type"
    );
    anyhow::ensure!(
        kv.get("ailake.index_type").unwrap() == "hnsw",
        "index_type deve ser hnsw"
    );

    println!("\n✓ AI Lake demo: Parquet colunar + footer HNSW (metadata) + pronto para");
    println!("  integração com Iceberg/Delta (Módulo 3) e busca vetorial (Módulo 7).");
    println!("  Ver Módulo 11, Projeto 1 — Mini Lakehouse vetorial para pipeline completo.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn footer_sobrevive_round_trip() {
        let dir = env::temp_dir().join("ailake_test");
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("test.parquet");
        let schema = schema_vendas();
        let batches = ler_csv(Path::new("dados/vendas.csv"), schema).expect("ok");
        let batch =
            arrow::compute::concat_batches(&batches[0].schema(), batches.iter()).expect("ok");
        escrever_parquet_com_footer(&out, &[batch]).expect("ok");
        let kv = ler_footer(&out).expect("ok").expect("deve ter metadata");
        assert_eq!(kv.get("ailake.index_type").unwrap(), "hnsw");
        assert_eq!(kv.get("ailake.format_version").unwrap(), "1.0");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn metadata_tem_campos_esperados() {
        let m = ailake_metadata();
        assert!(m.contains_key("ailake.vector_column"));
        assert!(m.contains_key("ailake.distance"));
        assert_eq!(m["ailake.hnsw_m"], "16");
    }
}
