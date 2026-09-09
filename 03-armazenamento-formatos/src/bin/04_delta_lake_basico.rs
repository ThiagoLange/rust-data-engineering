//! Módulo 03 — Armazenamento e Formatos
//! README: seção "Conteúdo", item 4 — Delta Lake básico
//!
//! Delta Lake adiciona transações ACID, versionamento e time travel sobre
//! arquivos Parquet. Este exemplo cria uma tabela Delta, faz append e lê uma
//! versão anterior.

use anyhow::{Context, Result};
use arrow::record_batch::RecordBatch;
use deltalake::kernel::{DataType as DeltaDataType, PrimitiveType, StructField};
use deltalake::operations::collect_sendable_stream;
use deltalake::protocol::SaveMode;
use deltalake::DeltaTable;
use std::path::{absolute, Path};
use url::Url;

use armazenamento_formatos::csv_utils::{ler_csv, schema_vendas};

fn path_para_url(caminho: &Path) -> Result<Url> {
    let abs = absolute(caminho).with_context(|| format!("resolvendo {}", caminho.display()))?;
    Url::from_directory_path(&abs)
        .map_err(|_| anyhow::anyhow!("caminho inválido para URL: {}", abs.display()))
}

fn colunas_delta() -> Vec<StructField> {
    vec![
        StructField::new(
            String::from("id"),
            DeltaDataType::Primitive(PrimitiveType::Long),
            false,
        ),
        StructField::new(
            String::from("data"),
            DeltaDataType::Primitive(PrimitiveType::String),
            false,
        ),
        StructField::new(
            String::from("categoria"),
            DeltaDataType::Primitive(PrimitiveType::String),
            false,
        ),
        StructField::new(
            String::from("produto"),
            DeltaDataType::Primitive(PrimitiveType::String),
            false,
        ),
        StructField::new(
            String::from("quantidade"),
            DeltaDataType::Primitive(PrimitiveType::Integer),
            false,
        ),
        StructField::new(
            String::from("preco_unitario"),
            DeltaDataType::Primitive(PrimitiveType::Double),
            false,
        ),
        StructField::new(
            String::from("regiao"),
            DeltaDataType::Primitive(PrimitiveType::String),
            false,
        ),
        StructField::new(
            String::from("cliente_id"),
            DeltaDataType::Primitive(PrimitiveType::Long),
            false,
        ),
    ]
}

async fn criar_tabela(caminho: &Path, _batch: RecordBatch) -> Result<DeltaTable> {
    let url = path_para_url(caminho)?;
    let table = DeltaTable::try_from_url(url)
        .await
        .context("abrindo local da tabela")?;

    table
        .create()
        .with_columns(colunas_delta())
        .with_table_name("vendas")
        .await
        .context("criando tabela")
}

async fn escrever(caminho: &Path, batches: Vec<RecordBatch>, modo: SaveMode) -> Result<DeltaTable> {
    let url = path_para_url(caminho)?;
    let table = DeltaTable::try_from_url(url)
        .await
        .context("abrindo tabela")?;

    table
        .write(batches)
        .with_save_mode(modo)
        .await
        .context("escrevendo na tabela")
}

async fn contar_linhas(caminho: &Path) -> Result<usize> {
    let url = path_para_url(caminho)?;
    let table = DeltaTable::try_from_url(url)
        .await
        .context("abrindo tabela para leitura")?;
    let (_table, stream) = table.scan_table().await.context("scan da tabela")?;
    let batches = collect_sendable_stream(stream)
        .await
        .context("coletando batches")?;
    Ok(batches.iter().map(|b| b.num_rows()).sum())
}

#[tokio::main]
async fn main() -> Result<()> {
    let entrada = Path::new("dados/vendas.csv");
    let saida = Path::new("dados/saida/delta_vendas");
    let saida_parent = saida
        .parent()
        .ok_or_else(|| anyhow::anyhow!("caminho sem parent: {}", saida.display()))?;
    std::fs::create_dir_all(saida_parent).context("criando diretório de saída")?;
    let _ = std::fs::remove_dir_all(saida);

    let schema = schema_vendas();
    let batches = ler_csv(entrada, schema)?;
    let inicial = vec![batches[0].clone()];

    let table = criar_tabela(saida, batches[0].clone()).await?;
    println!("Tabela Delta criada em: {}", saida.display());
    println!("Versão após create: {:?}", table.version());

    let table = escrever(saida, inicial, SaveMode::Overwrite).await?;
    println!("Versão após overwrite: {:?}", table.version());

    let append_batches = vec![batches[1].clone()];
    let table = escrever(saida, append_batches, SaveMode::Append).await?;
    println!("Versão após append: {:?}", table.version());

    let total = contar_linhas(saida).await?;
    println!("Total de linhas na tabela: {total}");

    Ok(())
}
