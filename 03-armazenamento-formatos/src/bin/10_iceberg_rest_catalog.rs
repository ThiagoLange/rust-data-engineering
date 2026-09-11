//! Módulo 03 — Armazenamento e Formatos
//! README: seção "Exemplos práticos", linha `10_iceberg_rest_catalog`.
//!
//! Mesmo fluxo do `08_iceberg_basico` (namespace → tabela particionada →
//! evolução de schema), mas contra um catálogo Iceberg **REST** de verdade
//! (`iceberg-catalog-rest`) em vez do `MemoryCatalog` local.
//!
//! Infra local (ver `docker-compose.yml` na raiz do módulo):
//! ```bash
//! docker compose up -d            # sobe tabulario/iceberg-rest-fixture em :8181
//! cargo run --bin 10_iceberg_rest_catalog
//! ```
//! A URI pode ser sobrescrita via `ICEBERG_REST_URI` (ex: Lakekeeper, Gravitino,
//! S3Tables) sem mudar o código — essa é a vantagem do catálogo REST: o cliente
//! fala HTTP, o servidor decide onde guardar metadata e quais engines o veem.

use anyhow::{Context, Result};
use iceberg::io::LocalFsStorageFactory;
use iceberg::spec::{NestedField, PrimitiveType, Schema, Transform, Type};
use iceberg::transaction::AddColumn;
use iceberg::transaction::ApplyTransactionAction;
use iceberg::{Catalog, CatalogBuilder, NamespaceIdent, TableCreation, TableIdent};
use iceberg_catalog_rest::{RestCatalogBuilder, REST_CATALOG_PROP_URI};
use std::collections::HashMap;
use std::sync::Arc;

/// URI do catálogo REST; sobrescrevível via ambiente para apontar
/// a qualquer implementação (fixture local, Lakekeeper, Gravitino...).
fn rest_uri() -> String {
    std::env::var("ICEBERG_REST_URI").unwrap_or_else(|_| "http://localhost:8181".to_string())
}

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

#[tokio::main]
async fn main() -> Result<()> {
    let uri = rest_uri();
    // FileIO client-side: o REST devolve `location` e o cliente lê/escreve
    // metadata JSON nela. `LocalFsStorageFactory` cobre `file://` (fixture local);
    // para S3/Azure/GCS, troque pela factory correspondente do `iceberg::io`.
    let catalog = RestCatalogBuilder::default()
        .with_storage_factory(Arc::new(LocalFsStorageFactory))
        .load(
            "rest",
            HashMap::from([(REST_CATALOG_PROP_URI.to_string(), uri.clone())]),
        )
        .await
        .with_context(|| {
            format!(
                "conectando ao catálogo REST em {uri} — suba com `docker compose up -d` na raiz do módulo"
            )
        })?;
    println!("Catálogo REST conectado em: {uri}");

    // 1. Namespace (idempotente: ignora se já existe)
    let ns = NamespaceIdent::from_strs(["vendas_db"]).context("namespace")?;
    match catalog.create_namespace(&ns, HashMap::new()).await {
        Ok(_) => println!("Namespace vendas_db criado"),
        Err(e) => println!("Namespace vendas_db já existe ou erro ignorado: {e}"),
    }
    let namespaces = catalog
        .list_namespaces(None)
        .await
        .context("listando namespaces")?;
    println!("Namespaces: {:?}", namespaces);

    // 2. Tabela particionada por `regiao` (hidden partitioning).
    // Sem `location`: o servidor REST decide onde guardar a metadata —
    // comportamento real de produção (no MemoryCatalog o cliente decidia).
    let table_ident = TableIdent::new(ns.clone(), "vendas_rest".to_string());
    let _ = catalog.drop_table(&table_ident).await;

    let schema = schema_vendas_iceberg()?;
    let partition_spec = iceberg::spec::PartitionSpec::builder(schema.clone())
        .with_spec_id(0)
        .add_partition_field("regiao", "regiao", Transform::Identity)
        .context("add partition field")?
        .build()
        .context("build partition spec")?;
    let creation = TableCreation::builder()
        .name("vendas_rest".to_string())
        .schema(schema.clone())
        .partition_spec(partition_spec)
        .build();

    let table = catalog
        .create_table(&ns, creation)
        .await
        .context("criando tabela Iceberg vendas_rest")?;
    println!("\nTabela criada: {}", table.identifier());
    println!(
        "  location (decidido pelo servidor): {}",
        table.metadata().location()
    );
    println!(
        "  schema-id atual: {}, partition-spec-id: {}",
        table.metadata().current_schema_id(),
        table.metadata().default_partition_spec_id()
    );

    // 3. Evolução de schema via commit transacional no servidor.
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
        "  fields: {}",
        table.metadata().current_schema().as_struct().fields().len()
    );

    // 4. Reload pelo catálogo — prova que o estado vive no servidor, não no processo.
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
    let tabelas = catalog.list_tables(&ns).await.context("listando tabelas")?;
    println!("Tabelas em vendas_db: {:?}", tabelas);

    println!("\n✓ Iceberg REST: namespace + tabela + evolução de schema via HTTP OK");
    println!("  Troque ICEBERG_REST_URI para falar com Lakekeeper/Gravitino/S3Tables sem mudar o código.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_tem_oito_campos() {
        let s = schema_vendas_iceberg().expect("ok");
        assert_eq!(s.as_struct().fields().len(), 8);
    }

    #[test]
    fn rest_uri_default_aponta_fixture_local() {
        // Sem variável de ambiente, o default é o docker-compose do módulo.
        // (Teste não abre conexão — o servidor é validado via `cargo run`.)
        std::env::remove_var("ICEBERG_REST_URI");
        assert_eq!(rest_uri(), "http://localhost:8181");
    }

    #[test]
    fn rest_uri_respeita_variavel_de_ambiente() {
        std::env::set_var("ICEBERG_REST_URI", "http://catalogo:8181");
        assert_eq!(rest_uri(), "http://catalogo:8181");
        std::env::remove_var("ICEBERG_REST_URI");
    }
}
