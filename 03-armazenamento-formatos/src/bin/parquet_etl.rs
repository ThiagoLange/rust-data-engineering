//! Módulo 03 — Armazenamento e Formatos
//! README: "Exemplo prático 1: Parquet particionado"
//!
//! Este exemplo simula um pipeline ETL simples: CSV → Parquet particionado por
//! região. O particionamento é feito manualmente criando diretórios no padrão
//! `regiao=XXX`, comum em data lakes.

use anyhow::{Context, Result};
use arrow::compute::{concat_batches, take};
use arrow::record_batch::RecordBatch;
use arrow_array::StringArray;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::arrow_writer::ArrowWriter;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use armazenamento_formatos::csv_utils::{ler_csv, schema_vendas};

fn particionar_por_regiao(batch: &RecordBatch) -> Result<HashMap<String, RecordBatch>> {
    let schema = batch.schema();
    let regioes = batch
        .column_by_name("regiao")
        .context("coluna regiao não encontrada")?
        .as_any()
        .downcast_ref::<StringArray>()
        .context("regiao não é string")?;

    let mut grupos: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, regiao) in regioes.iter().enumerate() {
        let regiao = regiao.context("regiao nula")?.to_string();
        grupos.entry(regiao).or_default().push(i);
    }

    let mut batches = HashMap::new();
    for (regiao, indices) in grupos {
        let indices: arrow_array::UInt32Array = indices.iter().map(|&i| Some(i as u32)).collect();
        let colunas: Vec<_> = batch
            .columns()
            .iter()
            .map(|col| take(col.as_ref(), &indices, None))
            .collect::<Result<Vec<_>, _>>()
            .context("filtrando colunas")?;
        let batch_filtrado =
            RecordBatch::try_new(schema.clone(), colunas).context("criando batch")?;
        batches.insert(regiao, batch_filtrado);
    }

    Ok(batches)
}

fn escrever_batch(caminho: &Path, batch: &RecordBatch) -> Result<()> {
    let file = File::create(caminho).with_context(|| format!("criando {}", caminho.display()))?;
    let mut writer = ArrowWriter::try_new(file, batch.schema(), None).context("writer")?;
    writer.write(batch).context("escrevendo")?;
    writer.close().context("finalizando")?;
    Ok(())
}

fn ler_parquets_recursivamente(dir: &Path) -> Result<Vec<RecordBatch>> {
    let mut batches = Vec::new();
    for entry in std::fs::read_dir(dir).context("lendo diretório")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            batches.extend(ler_parquets_recursivamente(&path)?);
        } else if path.extension().and_then(|s| s.to_str()) == Some("parquet") {
            let file = File::open(&path).with_context(|| format!("abrindo {}", path.display()))?;
            let reader = ParquetRecordBatchReaderBuilder::try_new(file)
                .context("reader")?
                .build()
                .context("build")?;
            batches.extend(reader.collect::<Result<Vec<_>, _>>().context("lendo")?);
        }
    }
    Ok(batches)
}

fn main() -> Result<()> {
    let entrada = Path::new("dados/vendas.csv");
    let saida_dir = Path::new("dados/saida/vendas_particionado");
    std::fs::create_dir_all(saida_dir).context("criando diretório de saída")?;

    let schema = schema_vendas();
    let batches = ler_csv(entrada, schema.clone())?;
    let batch_unico = concat_batches(&schema, batches.iter()).context("concatenando batches")?;

    let particoes = particionar_por_regiao(&batch_unico)?;
    for (regiao, batch) in &particoes {
        let dir = saida_dir.join(format!("regiao={regiao}"));
        std::fs::create_dir_all(&dir).context("criando partição")?;
        let caminho = dir.join("part-0.parquet");
        escrever_batch(&caminho, batch).with_context(|| format!("escrevendo {regiao}"))?;
        println!(
            "{regiao}: {} linhas em {}",
            batch.num_rows(),
            caminho.display()
        );
    }

    let lidos = ler_parquets_recursivamente(saida_dir)?;
    let total: usize = lidos.iter().map(|b| b.num_rows()).sum();
    println!("Total de linhas lidas de volta: {total}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn particionamento_preserva_linhas() {
        let schema = schema_vendas();
        let batches = ler_csv(Path::new("dados/vendas.csv"), schema.clone()).expect("ok");
        let batch = concat_batches(&schema, batches.iter()).expect("ok");
        let particoes = particionar_por_regiao(&batch).expect("ok");

        let total_particionado: usize = particoes.values().map(|b| b.num_rows()).sum();
        assert_eq!(total_particionado, batch.num_rows());
    }

    #[test]
    fn escrita_e_leitura_recursiva_funcionam() {
        let schema = schema_vendas();
        let batches = ler_csv(Path::new("dados/vendas.csv"), schema.clone()).expect("ok");
        let batch = concat_batches(&schema, batches.iter()).expect("ok");
        let particoes = particionar_por_regiao(&batch).expect("ok");

        let dir = env::temp_dir().join("parquet_etl_test");
        std::fs::create_dir_all(&dir).unwrap();
        for (regiao, batch) in &particoes {
            let d = dir.join(format!("regiao={regiao}"));
            std::fs::create_dir_all(&d).unwrap();
            escrever_batch(&d.join("part-0.parquet"), batch).unwrap();
        }

        let lidos = ler_parquets_recursivamente(&dir).expect("ok");
        let total: usize = lidos.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total, batch.num_rows());

        std::fs::remove_dir_all(dir).unwrap();
    }
}
