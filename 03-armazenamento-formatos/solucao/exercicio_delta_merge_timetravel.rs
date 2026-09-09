//! Módulo 03 — Armazenamento e Formatos
//! README: seção "Exercício"
//!
//! Pipeline incremental em tabela Delta particionada por `data`, com
//! deduplicação via `merge` (upsert por `id`) e validação de time travel.
//!
//! Etapas:
//! 1. Cria tabela Delta particionada por `data` e ingere `vendas.csv` (1k linhas).
//! 2. Aplica `merge` com `vendas_atualizacao.csv` (150 updates + 50 inserts).
//! 3. Lê versão 0/1 (antes do merge) e versão final via time travel e valida contagens.

use anyhow::{Context, Result};
use arrow::compute::concat_batches;
use arrow::record_batch::RecordBatch;
use datafusion::prelude::SessionContext;
use deltalake::kernel::{DataType as DeltaDataType, PrimitiveType, StructField};
use deltalake::operations::collect_sendable_stream;
use deltalake::protocol::SaveMode;
use deltalake::DeltaTable;
use std::path::{absolute, Path};
use url::Url;

use armazenamento_formatos::csv_utils::{ler_csv, schema_vendas};

// ---------------------------------------------------------------------------
// Helpers de Delta Lake
// ---------------------------------------------------------------------------

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

async fn criar_tabela_particionada(caminho: &Path) -> Result<DeltaTable> {
    // Garante que o diretório da tabela exista; caso contrário o LogStore falha
    // com "Path does not exist" ao tentar resolver a URL.
    std::fs::create_dir_all(caminho)
        .with_context(|| format!("criando diretório {}", caminho.display()))?;
    let url = path_para_url(caminho)?;
    let table = DeltaTable::try_from_url(url)
        .await
        .context("abrindo local da tabela")?;

    table
        .create()
        .with_columns(colunas_delta())
        .with_partition_columns(vec!["data"])
        .with_table_name("vendas")
        .await
        .context("criando tabela particionada")
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

async fn contar_linhas_versao(caminho: &Path, versao: i64) -> Result<usize> {
    let url = path_para_url(caminho)?;
    // time travel: abre a tabela numa versão específica
    let table = deltalake::open_table_with_version(url, versao as u64)
        .await
        .context("abrindo versão específica")?;
    let (_table, stream) = table.scan_table().await.context("scan versão")?;
    let batches = collect_sendable_stream(stream)
        .await
        .context("coletando batches versão")?;
    Ok(batches.iter().map(|b| b.num_rows()).sum())
}

fn batches_para_dataframe(batches: Vec<RecordBatch>) -> Result<datafusion::prelude::DataFrame> {
    if batches.is_empty() {
        anyhow::bail!("nenhum batch para criar DataFrame");
    }
    // Concatena todos os batches num único RecordBatch para o DataFrame.
    let schema = batches[0].schema();
    let batch = concat_batches(&schema, batches.iter()).context("concatenando batches")?;
    let ctx = SessionContext::new();
    ctx.read_batch(batch).context("criando DataFrame")
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> Result<()> {
    let entrada_inicial = Path::new("dados/vendas.csv");
    let entrada_atualizacao = Path::new("dados/vendas_atualizacao.csv");
    let tabela = Path::new("dados/saida/delta_vendas_exercicio");

    // Garante diretório pai e limpa execuções anteriores.
    let tabela_parent = tabela
        .parent()
        .ok_or_else(|| anyhow::anyhow!("caminho sem parent: {}", tabela.display()))?;
    std::fs::create_dir_all(tabela_parent).context("criando diretório de saída")?;
    let _ = std::fs::remove_dir_all(tabela);

    let schema = schema_vendas();

    // 1. Criação da tabela particionada por `data` + ingestão inicial
    let batches_inicial = ler_csv(entrada_inicial, schema.clone())?;
    let total_inicial: usize = batches_inicial.iter().map(|b| b.num_rows()).sum();
    println!("Linhas em vendas.csv: {total_inicial}");

    let table = criar_tabela_particionada(tabela).await?;
    println!(
        "Tabela criada (particionada por data) — versão {:?}",
        table.version()
    );

    let table = escrever(tabela, batches_inicial, SaveMode::Append).await?;
    println!("Versão após ingestão inicial: {:?}", table.version());

    let contagem_v1 = contar_linhas(tabela).await?;
    println!("Linhas após ingestão inicial: {contagem_v1}");
    anyhow::ensure!(
        contagem_v1 == 1000,
        "esperado 1000 linhas após ingestão inicial, obtido {contagem_v1}"
    );

    // 2. Ingestão incremental com deduplicação via merge (upsert por id)
    let batches_atualizacao = ler_csv(entrada_atualizacao, schema)?;
    let total_atualizacao: usize = batches_atualizacao.iter().map(|b| b.num_rows()).sum();
    println!("\nLinhas em vendas_atualizacao.csv: {total_atualizacao}");

    // Converte batches de atualização para DataFrame (source do merge)
    // Usa SessionContext separado para o source; o merge resolve o join
    // entre target (tabela Delta) e source (DataFrame).
    let source_df = batches_para_dataframe(batches_atualizacao)?;

    // Carrega a tabela atual para fazer o merge
    let url = path_para_url(tabela)?;
    let table = DeltaTable::try_from_url(url)
        .await
        .context("reabrindo para merge")?;

    // Dedup por `id`: se já existe, atualiza todas as colunas; senão, insere.
    // `target` e `source` são aliases obrigatórios para desambiguar colunas.
    use datafusion::logical_expr::{col, lit};

    let (table, metrics) = table
        .merge(source_df, col("target.id").eq(col("source.id")))
        .with_source_alias("source")
        .with_target_alias("target")
        .when_matched_update(|update| {
            update
                .update("data", col("source.data"))
                .update("categoria", col("source.categoria"))
                .update("produto", col("source.produto"))
                .update("quantidade", col("source.quantidade"))
                .update("preco_unitario", col("source.preco_unitario"))
                .update("regiao", col("source.regiao"))
                .update("cliente_id", col("source.cliente_id"))
        })?
        .when_not_matched_insert(|insert| {
            insert
                .set("id", col("source.id"))
                .set("data", col("source.data"))
                .set("categoria", col("source.categoria"))
                .set("produto", col("source.produto"))
                .set("quantidade", col("source.quantidade"))
                .set("preco_unitario", col("source.preco_unitario"))
                .set("regiao", col("source.regiao"))
                .set("cliente_id", col("source.cliente_id"))
        })?
        .await
        .context("executando merge")?;

    println!("\nMerge concluído — versão {:?}", table.version());
    println!(
        "  source_rows={}, updated={}, inserted={}, output_rows={}",
        metrics.num_source_rows,
        metrics.num_target_rows_updated,
        metrics.num_target_rows_inserted,
        metrics.num_output_rows
    );

    let contagem_final = contar_linhas(tabela).await?;
    println!("Linhas após merge: {contagem_final}");
    // 1000 iniciais + 50 novos (200 atualização - 150 overlap)
    anyhow::ensure!(
        contagem_final == 1050,
        "esperado 1050 linhas após merge, obtido {contagem_final}"
    );

    // 3. Time travel — valida estado antes/depois do merge
    // Versões: 0 = create, 1 = ingestão inicial, 2 = merge
    let v1 = contar_linhas_versao(tabela, 1).await?;
    let v2 = contar_linhas_versao(tabela, 2).await?;
    println!("\n=== Time travel ===");
    println!("  versão 1 (antes do merge): {v1} linhas");
    println!("  versão 2 (após  merge)   : {v2} linhas");

    anyhow::ensure!(v1 == 1000, "time travel v1 deve ter 1000, tem {v1}");
    anyhow::ensure!(v2 == 1050, "time travel v2 deve ter 1050, tem {v2}");
    anyhow::ensure!(v2 > v1, "versão após merge deve ter mais linhas que antes");

    println!("\n✓ Pipeline incremental validado com time travel.");

    // Evita warning de variável não usada
    let _ = lit(1);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn merge_dedup_e_timetravel() {
        let dir = env::temp_dir().join("delta_merge_exercicio_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("ok");
        let tabela = dir.join("tabela_delta");

        let schema = schema_vendas();
        let batches = ler_csv(Path::new("dados/vendas.csv"), schema.clone()).expect("ok");
        let batches_atualizacao =
            ler_csv(Path::new("dados/vendas_atualizacao.csv"), schema).expect("ok");

        // cria + ingestão inicial
        let table = criar_tabela_particionada(&tabela).await.expect("ok");
        assert_eq!(table.version(), Some(0));
        let table = escrever(&tabela, batches, SaveMode::Append)
            .await
            .expect("ok");
        assert_eq!(table.version(), Some(1));
        let c1 = contar_linhas(&tabela).await.expect("ok");
        assert_eq!(c1, 1000);

        // merge
        let source_df = batches_para_dataframe(batches_atualizacao).expect("ok");
        use datafusion::logical_expr::col;
        let url = path_para_url(&tabela).expect("ok");
        let table = DeltaTable::try_from_url(url).await.expect("ok");
        let (table, _metrics) = table
            .merge(source_df, col("target.id").eq(col("source.id")))
            .with_source_alias("source")
            .with_target_alias("target")
            .when_matched_update(|u| {
                u.update("data", col("source.data"))
                    .update("categoria", col("source.categoria"))
                    .update("produto", col("source.produto"))
                    .update("quantidade", col("source.quantidade"))
                    .update("preco_unitario", col("source.preco_unitario"))
                    .update("regiao", col("source.regiao"))
                    .update("cliente_id", col("source.cliente_id"))
            })
            .expect("ok")
            .when_not_matched_insert(|i| {
                i.set("id", col("source.id"))
                    .set("data", col("source.data"))
                    .set("categoria", col("source.categoria"))
                    .set("produto", col("source.produto"))
                    .set("quantidade", col("source.quantidade"))
                    .set("preco_unitario", col("source.preco_unitario"))
                    .set("regiao", col("source.regiao"))
                    .set("cliente_id", col("source.cliente_id"))
            })
            .expect("ok")
            .await
            .expect("ok");

        assert_eq!(table.version(), Some(2));
        let c2 = contar_linhas(&tabela).await.expect("ok");
        assert_eq!(c2, 1050);

        // time travel
        let v1 = contar_linhas_versao(&tabela, 1).await.expect("ok");
        let v2 = contar_linhas_versao(&tabela, 2).await.expect("ok");
        assert_eq!(v1, 1000);
        assert_eq!(v2, 1050);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn schema_tem_coluna_data_para_particionamento() {
        let cols = colunas_delta();
        assert!(cols.iter().any(|c| c.name() == "data"));
        assert!(cols.iter().any(|c| c.name() == "id"));
    }

    #[test]
    fn batches_para_dataframe_preserva_linhas() {
        let schema = schema_vendas();
        let batches = ler_csv(Path::new("dados/vendas.csv"), schema).expect("ok");
        let total: usize = batches.iter().map(|b| b.num_rows()).sum();
        let df = batches_para_dataframe(batches).expect("ok");
        // DataFrame deve ter mesmo número de linhas
        let rt = tokio::runtime::Runtime::new().expect("ok");
        let collected = rt.block_on(df.collect()).expect("ok");
        let total_df: usize = collected.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total, total_df);
    }
}
