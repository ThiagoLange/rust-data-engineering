//! Módulo 03 — Armazenamento e Formatos
//! README: seção "Conteúdo", item 2 — Parquet: schema e compressão
//!
//! Parquet permite definir o schema explicitamente e escolher algoritmos de
//! compressão por coluna. Este exemplo compara os tamanhos de arquivo com
//! compressões diferentes: nenhuma, Snappy e Zstd.

use anyhow::{Context, Result};
use arrow::record_batch::RecordBatch;
use arrow_schema::SchemaRef;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::path::Path;

use armazenamento_formatos::csv_utils::{ler_csv, schema_vendas};

fn escrever_com_compressao(
    caminho: &Path,
    schema: SchemaRef,
    batches: &[RecordBatch],
    compression: Compression,
) -> Result<u64> {
    let props = WriterProperties::builder()
        .set_compression(compression)
        .build();
    let file = File::create(caminho).with_context(|| format!("criando {}", caminho.display()))?;
    let mut writer = ArrowWriter::try_new(file, schema, Some(props)).context("criando writer")?;

    for batch in batches {
        writer.write(batch).context("escrevendo batch")?;
    }
    writer.close().context("finalizando parquet")?;

    Ok(std::fs::metadata(caminho)
        .with_context(|| format!("lendo metadados de {}", caminho.display()))?
        .len())
}

fn main() -> Result<()> {
    let entrada = Path::new("dados/vendas.csv");
    let saida_dir = Path::new("dados/saida/compressao");
    std::fs::create_dir_all(saida_dir).context("criando diretório de saída")?;

    let schema = schema_vendas();
    let batches = ler_csv(entrada, schema.clone())?;

    let tamanhos = vec![
        (
            "uncompressed",
            escrever_com_compressao(
                &saida_dir.join("vendas_uncompressed.parquet"),
                schema.clone(),
                &batches,
                Compression::UNCOMPRESSED,
            )?,
        ),
        (
            "snappy",
            escrever_com_compressao(
                &saida_dir.join("vendas_snappy.parquet"),
                schema.clone(),
                &batches,
                Compression::SNAPPY,
            )?,
        ),
        (
            "zstd",
            escrever_com_compressao(
                &saida_dir.join("vendas_zstd.parquet"),
                schema.clone(),
                &batches,
                Compression::ZSTD(Default::default()),
            )?,
        ),
    ];

    println!("Tamanhos por compressão:");
    for (nome, bytes) in tamanhos {
        println!("  {nome:12} -> {bytes:8} bytes");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn tres_arquivos_sao_gerados() {
        let schema = schema_vendas();
        let batches = ler_csv(Path::new("dados/vendas.csv"), schema.clone()).expect("ok");
        let dir = env::temp_dir().join("parquet_compressao_test");
        std::fs::create_dir_all(&dir).unwrap();

        let uncompressed = escrever_com_compressao(
            &dir.join("u.parquet"),
            schema.clone(),
            &batches,
            Compression::UNCOMPRESSED,
        )
        .expect("ok");
        let snappy = escrever_com_compressao(
            &dir.join("s.parquet"),
            schema.clone(),
            &batches,
            Compression::SNAPPY,
        )
        .expect("ok");
        let zstd = escrever_com_compressao(
            &dir.join("z.parquet"),
            schema,
            &batches,
            Compression::ZSTD(Default::default()),
        )
        .expect("ok");

        assert!(uncompressed >= snappy);
        assert!(snappy >= zstd);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
