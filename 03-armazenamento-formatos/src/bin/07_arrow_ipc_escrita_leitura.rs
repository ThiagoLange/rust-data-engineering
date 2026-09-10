//! Módulo 03 — Armazenamento e Formatos
//! README: seção "Conteúdo", item 3 — Arrow IPC
//!
//! Arrow IPC é o formato para transferência rápida entre processos
//! (memória compartilhada, mmap) e streaming. Este exemplo serializa
//! `RecordBatch` em IPC File (seekable) e Stream (pipe), lê de volta via
//! `FileReader`/`StreamReader`, e compara tamanhos.

use anyhow::{Context, Result};
use arrow::record_batch::RecordBatch;
use arrow_array::StringArray;
use arrow_ipc::reader::{FileReader, StreamReader};
use arrow_ipc::writer::{FileWriter, StreamWriter};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use armazenamento_formatos::csv_utils::{ler_csv, schema_vendas};

/// Escreve `batches` em Arrow IPC File.
fn escrever_ipc_file(caminho: &Path, batches: &[RecordBatch]) -> Result<usize> {
    if batches.is_empty() {
        anyhow::bail!("nenhum batch para escrever");
    }
    let file = File::create(caminho).with_context(|| format!("criando {}", caminho.display()))?;
    let mut writer =
        FileWriter::try_new(file, &batches[0].schema()).context("criando FileWriter IPC")?;
    for batch in batches {
        writer.write(batch).context("escrevendo batch IPC")?;
    }
    writer.finish().context("finalizando IPC file")?;
    Ok(std::fs::metadata(caminho)
        .with_context(|| format!("metadata {}", caminho.display()))?
        .len() as usize)
}

/// Escreve `batches` em Arrow IPC Stream (não seekable, ideal para pipes/Kafka).
fn escrever_ipc_stream(caminho: &Path, batches: &[RecordBatch]) -> Result<usize> {
    if batches.is_empty() {
        anyhow::bail!("nenhum batch para escrever");
    }
    let file = File::create(caminho).with_context(|| format!("criando {}", caminho.display()))?;
    let mut writer =
        StreamWriter::try_new(file, &batches[0].schema()).context("criando StreamWriter IPC")?;
    for batch in batches {
        writer.write(batch).context("escrevendo batch stream")?;
    }
    writer.finish().context("finalizando IPC stream")?;
    Ok(std::fs::metadata(caminho)
        .with_context(|| format!("metadata {}", caminho.display()))?
        .len() as usize)
}

fn ler_ipc_file(caminho: &Path) -> Result<Vec<RecordBatch>> {
    let file = File::open(caminho).with_context(|| format!("abrindo {}", caminho.display()))?;
    let reader = FileReader::try_new(file, None).context("criando FileReader IPC")?;
    reader
        .collect::<Result<Vec<_>, _>>()
        .context("lendo batches IPC file")
}

fn ler_ipc_stream(caminho: &Path) -> Result<Vec<RecordBatch>> {
    let file = File::open(caminho).with_context(|| format!("abrindo {}", caminho.display()))?;
    let reader = StreamReader::try_new(file, None).context("criando StreamReader IPC")?;
    reader
        .collect::<Result<Vec<_>, _>>()
        .context("lendo batches IPC stream")
}

/// Demonstra zero-copy via `mmap` (se o arquivo fosse mmaped).
/// Aqui apenas valida que os dados lidos são idênticos — o principio de
/// zero-copy seria aplicado com `memmap2` em produção, fora do escopo do exemplo.
fn validar_batches(orig: &[RecordBatch], lidos: &[RecordBatch]) -> Result<()> {
    let total_orig: usize = orig.iter().map(|b| b.num_rows()).sum();
    let total_lidos: usize = lidos.iter().map(|b| b.num_rows()).sum();
    anyhow::ensure!(
        total_orig == total_lidos,
        "contagem divergente: {total_orig} vs {total_lidos}"
    );
    // Valida que a coluna `regiao` mantém os valores
    let orig_regioes: Vec<String> = orig
        .iter()
        .flat_map(|b| {
            b.column_by_name("regiao")
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .iter()
                .map(|v| v.unwrap().to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    let lidas_regioes: Vec<String> = lidos
        .iter()
        .flat_map(|b| {
            b.column_by_name("regiao")
                .unwrap()
                .as_any()
                .downcast_ref::<StringArray>()
                .unwrap()
                .iter()
                .map(|v| v.unwrap().to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    anyhow::ensure!(
        orig_regioes == lidas_regioes,
        "valores de regiao divergentes"
    );
    Ok(())
}

fn main() -> Result<()> {
    let entrada = Path::new("dados/vendas.csv");
    let saida_dir = Path::new("dados/saida/arrow_ipc");
    let saida_file = saida_dir.join("vendas.arrow");
    let saida_stream = saida_dir.join("vendas.stream.arrow");

    let saida_parent = saida_file
        .parent()
        .ok_or_else(|| anyhow::anyhow!("caminho sem parent: {}", saida_file.display()))?;
    std::fs::create_dir_all(saida_parent).context("criando diretório de saída")?;

    let schema = schema_vendas();
    let batches = ler_csv(entrada, schema)?;
    let total: usize = batches.iter().map(|b| b.num_rows()).sum();
    println!("Lidos {total} linhas em {} batches", batches.len());

    let tam_file = escrever_ipc_file(&saida_file, &batches)?;
    let tam_stream = escrever_ipc_stream(&saida_stream, &batches)?;
    println!("IPC File   : {tam_file} bytes -> {}", saida_file.display());
    println!(
        "IPC Stream : {tam_stream} bytes -> {}",
        saida_stream.display()
    );

    let lidos_file = ler_ipc_file(&saida_file)?;
    let lidos_stream = ler_ipc_stream(&saida_stream)?;
    println!(
        "Lidos File {} batches, Stream {} batches",
        lidos_file.len(),
        lidos_stream.len()
    );

    validar_batches(&batches, &lidos_file).context("validando file")?;
    validar_batches(&batches, &lidos_stream).context("validando stream")?;
    println!("Round-trip IPC OK (file e stream)");

    // Demonstra leitura parcial / projeção — IPC File permite acesso por Footer
    let schema_lido = lidos_file[0].schema();
    println!("Schema preservado: {}", schema_lido);

    // Compara com Parquet para contexto pedagógico
    println!("\nNota: IPC é para transporte em memória (zero-copy, mmap);");
    println!("Parquet é para armazenamento colunar comprimido e durável.");
    println!("Em pipelines, use IPC entre processos e Parquet no Lake.");

    // Evita warning de import não usado
    let _ = Arc::new(1);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn ipc_file_round_trip() {
        let dir = env::temp_dir().join("ipc_file_test");
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("test.arrow");
        let schema = schema_vendas();
        let batches = ler_csv(Path::new("dados/vendas.csv"), schema).expect("ok");
        let batches_slice = &batches[..1]; // 1 batch é suficiente para teste rápido
        escrever_ipc_file(&out, batches_slice).expect("ok");
        let lidos = ler_ipc_file(&out).expect("ok");
        validar_batches(batches_slice, &lidos).expect("ok");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ipc_stream_round_trip() {
        let dir = env::temp_dir().join("ipc_stream_test");
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("test.stream.arrow");
        let schema = schema_vendas();
        let batches = ler_csv(Path::new("dados/vendas.csv"), schema).expect("ok");
        let batches_slice = &batches[..1];
        escrever_ipc_stream(&out, batches_slice).expect("ok");
        let lidos = ler_ipc_stream(&out).expect("ok");
        validar_batches(batches_slice, &lidos).expect("ok");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn ipc_file_e_stream_tem_mesmo_numero_de_linhas() {
        let dir = env::temp_dir().join("ipc_compare");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("f.arrow");
        let s = dir.join("s.arrow");
        let schema = schema_vendas();
        let batches = ler_csv(Path::new("dados/vendas.csv"), schema).expect("ok");
        escrever_ipc_file(&f, &batches).expect("ok");
        escrever_ipc_stream(&s, &batches).expect("ok");
        let lf = ler_ipc_file(&f).expect("ok");
        let ls = ler_ipc_stream(&s).expect("ok");
        let cf: usize = lf.iter().map(|b| b.num_rows()).sum();
        let cs: usize = ls.iter().map(|b| b.num_rows()).sum();
        assert_eq!(cf, cs);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
