//! Módulo 02 — Processamento de Dados
//! README: seção "Conteúdo", item 2 — DataFusion: SQL embutido e UDFs
//!
//! DataFusion é um motor de query SQL/Query Planner construído sobre Apache
//! Arrow. Ele permite registrar arquivos CSV/Parquet como tabelas e executar
//! SQL diretamente sobre eles — útil quando você quer expor uma interface SQL
//! para usuários ou conectar várias fontes de dados.
//!
//! UDFs (User Defined Functions) permitem registrar funções escritas em Rust
//! e chamálas dentro do SQL, estendendo o motor com lógica customizada.

use datafusion::arrow::array::StringArray;
use datafusion::arrow::datatypes::DataType;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{create_udf, ColumnarValue, Volatility};
use datafusion::prelude::*;
use std::sync::Arc;

/// Registra o CSV de transações como uma tabela chamada "transacoes" e executa
/// uma query SQL simples.
async fn total_por_categoria(ctx: &SessionContext) -> anyhow::Result<()> {
    let df = ctx
        .sql(
            "SELECT categoria, SUM(valor) AS total, COUNT(*) AS quantidade \
             FROM transacoes \
             GROUP BY categoria \
             ORDER BY total DESC",
        )
        .await?;
    df.show().await?;
    Ok(())
}

/// UDF que normaliza um texto: minúsculas e sem acentos simples.
/// Em produção, usar uma biblioteca de normalização; aqui é didático.
fn normalizar_texto(args: &[ColumnarValue]) -> Result<ColumnarValue, DataFusionError> {
    let input = match &args[0] {
        ColumnarValue::Array(array) => array
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| DataFusionError::Execution("esperava array de strings".into()))?,
        ColumnarValue::Scalar(_) => {
            return Err(DataFusionError::Execution(
                "UDF normalizar não suporta scalar".into(),
            ));
        }
    };

    let resultado: Vec<_> = input
        .iter()
        .map(|opt| {
            opt.map(|s| {
                s.to_lowercase()
                    .replace(['á', 'à', 'â', 'ã'], "a")
                    .replace(['é', 'è', 'ê'], "e")
                    .replace(['í', 'ì', 'î'], "i")
                    .replace(['ó', 'ò', 'ô', 'õ'], "o")
                    .replace(['ú', 'ù', 'û'], "u")
                    .replace('ç', "c")
            })
        })
        .collect();

    Ok(ColumnarValue::Array(Arc::new(StringArray::from(resultado))))
}

/// Registra a UDF no contexto do DataFusion.
fn registrar_udf_normalizar(ctx: &SessionContext) {
    let udf = create_udf(
        "normalizar",
        vec![DataType::Utf8],
        DataType::Utf8,
        Volatility::Immutable,
        Arc::new(normalizar_texto),
    );
    ctx.register_udf(udf);
}

/// Executa uma query usando a UDF customizada.
async fn categorias_normalizadas(ctx: &SessionContext) -> anyhow::Result<()> {
    let df = ctx
        .sql("SELECT DISTINCT normalizar(categoria) AS categoria_norm FROM transacoes")
        .await?;
    df.show().await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let ctx = SessionContext::new();

    // Registra o CSV como tabela. DataFusion infere o schema a partir dos dados.
    ctx.register_csv("transacoes", "dados/transacoes.csv", CsvReadOptions::new())
        .await?;

    println!("=== Total por categoria (SQL) ===");
    total_por_categoria(&ctx).await?;

    println!("\n=== UDF normalizar(categoria) ===");
    registrar_udf_normalizar(&ctx);
    categorias_normalizadas(&ctx).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn datafusion_le_csv_e_executa_sql() {
        let ctx = SessionContext::new();
        ctx.register_csv("transacoes", "dados/transacoes.csv", CsvReadOptions::new())
            .await
            .expect("ok");

        let df = ctx
            .sql("SELECT COUNT(*) AS total FROM transacoes")
            .await
            .expect("ok");
        let batches = df.collect().await.expect("ok");
        assert_eq!(batches.len(), 1);
    }

    #[test]
    fn udf_normalizar_funciona() {
        let input = Arc::new(StringArray::from(vec!["Eletrônicos", "AÇÃO"]));
        let resultado = normalizar_texto(&[ColumnarValue::Array(input)]).expect("ok");
        match resultado {
            ColumnarValue::Array(array) => {
                let strings = array
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .expect("array de strings");
                assert_eq!(strings.value(0), "eletronicos");
                assert_eq!(strings.value(1), "acao");
            }
            ColumnarValue::Scalar(_) => panic!("esperava array"),
        }
    }
}
