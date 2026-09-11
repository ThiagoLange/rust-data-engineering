//! Módulo 03 — Armazenamento e Formatos
//! README: seção "Conteúdo", item 5 — Iceberg
//!
//! Apache Iceberg v2 com `iceberg` crate: catalog, schema com particionamento
//! oculto, evolução de schema sem reescrita. Demonstra criação de namespace,
//! criação de tabela, inspeção de metadata e evolução de schema.

use anyhow::{Context, Result};
use iceberg::memory::{MemoryCatalogBuilder, MEMORY_CATALOG_WAREHOUSE};
use iceberg::spec::{NestedField, PrimitiveType, Schema, Transform, Type};
use iceberg::transaction::AddColumn;
use iceberg::transaction::ApplyTransactionAction;
use iceberg::{Catalog, CatalogBuilder, NamespaceIdent, TableCreation, TableIdent};
use std::collections::HashMap;
use std::path::Path;

fn schema_vendas_iceberg() -> Result<Schema> {
    // IDs sequenciais — Iceberg exige IDs únicos e estáveis
    Schema::builder()
        .with_fields(vec![
            NestedField::required(1, "id", Type::Primitive(PrimitiveType::Long)).into(),
            NestedField::required(2, "data", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::required(3, "categoria", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::required(4, "produto", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::required(5, "quantidade", Type::Primitive(PrimitiveType::Int)).into(),
            NestedField::required(6, "preco_unitario", Type::Primitive(PrimitiveType::Double))
                .into(),
            NestedField::required(7, "regiao", Type::Primitive(PrimitiveType::String)).into(),
            NestedField::required(8, "cliente_id", Type::Primitive(PrimitiveType::Long)).into(),
        ])
        .build()
        .context("construindo schema Iceberg")
}

fn criar_catalogo_warehouse(path: &Path) -> Result<String> {
    std::fs::create_dir_all(path)
        .with_context(|| format!("criando warehouse {}", path.display()))?;
    Ok(path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    let warehouse = Path::new("dados/saida/iceberg_warehouse");
    let warehouse_str = criar_catalogo_warehouse(warehouse)?;
    let catalog = MemoryCatalogBuilder::default()
        .load(
            "demo",
            HashMap::from([(MEMORY_CATALOG_WAREHOUSE.to_string(), warehouse_str.clone())]),
        )
        .await
        .context("criando MemoryCatalog")?;
    println!("Catalog Memory criado em: {}", warehouse.display());

    // 1. Namespace
    let ns = NamespaceIdent::from_strs(["vendas_db"]).context("namespace")?;
    // `create_namespace` falha se já existe — ignora erro de já existente
    match catalog.create_namespace(&ns, HashMap::new()).await {
        Ok(_) => println!("Namespace vendas_db criado"),
        Err(e) => println!("Namespace vendas_db já existe ou erro ignorado: {e}"),
    }

    let namespaces = catalog
        .list_namespaces(None)
        .await
        .context("listando namespaces")?;
    println!("Namespaces: {:?}", namespaces);

    // 2. Schema
    let schema = schema_vendas_iceberg()?;
    println!("\nSchema Iceberg:\n{schema:?}");

    // 3. Criação da tabela com particionamento oculto por `regiao` (identity)
    let table_ident = TableIdent::new(ns.clone(), "vendas".to_string());
    // Remove se já existe (para re-execução idempotente)
    let _ = catalog.drop_table(&table_ident).await;

    let partition_spec = {
        // Usa API `add_partition_field(source_name, target_name, transform)`
        // — particionamento oculto: o usuário filtra por `regiao`, o engine
        // resolve para a partição física sem expor `regiao=XX` no SQL.
        let builder = iceberg::spec::PartitionSpec::builder(schema.clone()).with_spec_id(0);
        builder
            .add_partition_field("regiao", "regiao", Transform::Identity)
            .context("add partition field")?
            .build()
            .context("build partition spec")?
    };
    println!("\nPartitionSpec (hidden partitioning por regiao):\n{partition_spec:?}");

    let creation = TableCreation::builder()
        .name("vendas".to_string())
        .schema(schema.clone())
        .partition_spec(partition_spec)
        .location(format!("{warehouse_str}/vendas_db/vendas"))
        .build();

    let table = catalog
        .create_table(&ns, creation)
        .await
        .context("criando tabela Iceberg vendas")?;
    println!("\nTabela criada: {}", table.identifier());

    // 4. Inspeção de metadata (snapshot v0 — sem dados ainda)
    let metadata = table.metadata();
    println!(
        "  schema-id atual: {}, partition-spec-id: {}",
        metadata.current_schema_id(),
        metadata.default_partition_spec_id()
    );
    println!("  snapshots: {}", metadata.snapshots().count());
    println!("  current_snapshot: {:?}", metadata.current_snapshot());

    // 5. Evolução de schema — adiciona coluna opcional `desconto`
    // Demonstra que Iceberg permite evolução sem reescrever dados.
    use iceberg::transaction::Transaction;
    let tx = Transaction::new(&table);
    let tx = tx
        .update_schema()
        .add_column(
            AddColumn::builder()
                .name("desconto")
                .field_type(Type::Primitive(PrimitiveType::Double))
                .doc("desconto aplicado na venda")
                .build(),
        )
        .apply(tx)
        .context("aplicando evolução de schema")?;
    let table = tx.commit(&catalog).await.context("commit evolução")?;
    println!("\nApós evolução de schema (add coluna `desconto`):");
    println!(
        "  schema: {:?}",
        table.metadata().current_schema().as_struct()
    );
    println!(
        "  total snapshots ainda: {}",
        table.metadata().snapshots().count()
    );

    // 6. Listagem de tabelas
    let tabelas = catalog.list_tables(&ns).await.context("listando tabelas")?;
    println!("\nTabelas em vendas_db: {:?}", tabelas);

    // 7. Carrega novamente via `load_table` (time travel conceitual: acessar metadata)
    let table_reload = catalog
        .load_table(&table_ident)
        .await
        .context("load_table")?;
    println!(
        "\nReload OK — schema fields: {}",
        table_reload
            .metadata()
            .current_schema()
            .as_struct()
            .fields()
            .len()
    );

    println!("\n✓ Iceberg: catalog + namespace + tabela + particionamento oculto + evolução de schema OK");
    println!("  Nota AI Lake: este padrão (Parquet + metadata Iceberg + índice vetorial HNSW)");
    println!("  é a base do formato AI-Lake — ver Módulo 11, Projeto 1.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_warehouse() -> TempDir {
        TempDir::new().unwrap()
    }

    async fn new_catalog(dir: &std::path::Path) -> iceberg::memory::MemoryCatalog {
        let wh = criar_catalogo_warehouse(dir).unwrap();
        MemoryCatalogBuilder::default()
            .load(
                "test",
                HashMap::from([(MEMORY_CATALOG_WAREHOUSE.to_string(), wh)]),
            )
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn cria_catalogo_e_tabela() {
        let dir = temp_warehouse();
        let catalog = new_catalog(dir.path()).await;
        let ns = NamespaceIdent::from_strs(["test_ns"]).unwrap();
        catalog
            .create_namespace(&ns, HashMap::new())
            .await
            .expect("ok");
        let schema = schema_vendas_iceberg().expect("ok");
        let ps = iceberg::spec::PartitionSpec::builder(schema.clone())
            .with_spec_id(0)
            .add_partition_field("regiao", "regiao", Transform::Identity)
            .unwrap()
            .build()
            .unwrap();
        let ident = TableIdent::new(ns.clone(), "t1".to_string());
        let creation = TableCreation::builder()
            .name("t1".to_string())
            .schema(schema)
            .partition_spec(ps)
            .build();
        let table = catalog.create_table(&ns, creation).await.expect("ok");
        assert_eq!(table.identifier().name(), "t1");
        catalog.drop_table(&ident).await.expect("ok");
    }

    #[tokio::test]
    async fn evolucao_de_schema_adiciona_coluna() {
        let dir = temp_warehouse();
        let catalog = new_catalog(dir.path()).await;
        let ns = NamespaceIdent::from_strs(["ns2"]).unwrap();
        catalog
            .create_namespace(&ns, HashMap::new())
            .await
            .expect("ok");
        let schema = schema_vendas_iceberg().expect("ok");
        let ps = iceberg::spec::PartitionSpec::builder(schema.clone())
            .with_spec_id(0)
            .add_partition_field("regiao", "regiao", Transform::Identity)
            .unwrap()
            .build()
            .unwrap();
        let _ident = TableIdent::new(ns.clone(), "vendas".to_string());
        let creation = TableCreation::builder()
            .name("vendas".to_string())
            .schema(schema)
            .partition_spec(ps)
            .build();
        let table = catalog.create_table(&ns, creation).await.expect("ok");
        let campos_antes = table.metadata().current_schema().as_struct().fields().len();
        use iceberg::transaction::Transaction;
        let tx = Transaction::new(&table);
        let tx = tx
            .update_schema()
            .add_column(AddColumn::optional(
                "nova_col",
                Type::Primitive(PrimitiveType::String),
            ))
            .apply(tx)
            .expect("ok");
        let table = tx.commit(&catalog).await.expect("ok");
        assert_eq!(
            table.metadata().current_schema().as_struct().fields().len(),
            campos_antes + 1
        );
    }

    #[test]
    fn schema_tem_oito_campos() {
        let s = schema_vendas_iceberg().expect("ok");
        assert_eq!(s.as_struct().fields().len(), 8);
    }
}
