//! Módulo 02 — Processamento de Dados
//! README: seção "Conteúdo", item 1 — Group-by, joins e agregações com Polars
//!
//! Este exemplo mostra operações típicas de transformação de dados:
//! - agrupar e agregar (group-by + sum/mean/count)
//! - juntar dois DataFrames por uma chave comum (join)
//! - criar colunas derivadas com expressões (`when/then/otherwise`)
//!
//! Tudo é feito de forma lazy: o plano inteiro é otimizado antes da execução.

use polars::prelude::*;
use std::path::Path;

/// Carrega as transações como LazyFrame.
fn ler_transacoes(caminho: &Path) -> anyhow::Result<LazyFrame> {
    Ok(
        LazyCsvReader::new(caminho.to_string_lossy().as_ref().into())
            .with_has_header(true)
            .finish()?,
    )
}

/// Carrega os clientes como LazyFrame.
fn ler_clientes(caminho: &Path) -> anyhow::Result<LazyFrame> {
    Ok(
        LazyCsvReader::new(caminho.to_string_lossy().as_ref().into())
            .with_has_header(true)
            .finish()?,
    )
}

/// Total de vendas por categoria.
fn vendas_por_categoria(lf: LazyFrame) -> anyhow::Result<DataFrame> {
    lf.group_by([col("categoria")])
        .agg([
            col("valor").sum().alias("total"),
            col("valor").mean().alias("media"),
            col("id").count().alias("quantidade"),
        ])
        .sort(
            ["total"],
            SortMultipleOptions::default().with_order_descending(true),
        )
        .collect()
        .map_err(Into::into)
}

/// Junta transações com clientes para mostrar o total gasto por região.
fn vendas_por_regiao(transacoes: LazyFrame, clientes: LazyFrame) -> anyhow::Result<DataFrame> {
    transacoes
        .join(
            clientes,
            [col("cliente_id")],
            [col("cliente_id")],
            JoinType::Inner.into(),
        )
        .group_by([col("regiao")])
        .agg([col("valor").sum().alias("total_vendas")])
        .sort(
            ["total_vendas"],
            SortMultipleOptions::default().with_order_descending(true),
        )
        .collect()
        .map_err(Into::into)
}

/// Cria uma coluna de desconto: 10% para vendas acima de 500, 5% entre 100 e 500.
fn calcular_desconto(lf: LazyFrame) -> anyhow::Result<DataFrame> {
    lf.with_columns([(when(col("valor").gt(lit(500.0)))
        .then(col("valor") * lit(0.10))
        .when(col("valor").gt(lit(100.0)))
        .then(col("valor") * lit(0.05))
        .otherwise(lit(0.0)))
    .alias("desconto")])
        .with_columns([(col("valor") - col("desconto")).alias("valor_final")])
        .collect()
        .map_err(Into::into)
}

fn main() -> anyhow::Result<()> {
    let transacoes = ler_transacoes(Path::new("dados/transacoes.csv"))?;
    let clientes = ler_clientes(Path::new("dados/clientes.csv"))?;

    println!("=== Vendas por categoria ===");
    println!("{}", vendas_por_categoria(transacoes.clone())?);

    println!("\n=== Vendas por região ===");
    println!("{}", vendas_por_regiao(transacoes.clone(), clientes)?);

    println!("\n=== Exemplo de desconto ===");
    println!("{}", calcular_desconto(transacoes)?.head(Some(10)));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendas_por_categoria_retorna_todas_as_categorias() {
        let lf = ler_transacoes(Path::new("dados/transacoes.csv")).expect("ok");
        let df = vendas_por_categoria(lf).expect("ok");
        assert_eq!(df.height(), 5); // eletronicos, livros, alimentos, roupas, servicos
    }

    #[test]
    fn join_com_clientes_cria_coluna_regiao() {
        let transacoes = ler_transacoes(Path::new("dados/transacoes.csv")).expect("ok");
        let clientes = ler_clientes(Path::new("dados/clientes.csv")).expect("ok");
        let df = vendas_por_regiao(transacoes, clientes).expect("ok");

        assert!(df
            .get_column_names()
            .iter()
            .any(|nome| nome.as_str() == "regiao"));
        assert!(df
            .get_column_names()
            .iter()
            .any(|nome| nome.as_str() == "total_vendas"));
        assert_eq!(df.height(), 5); // 5 regiões
    }

    #[test]
    fn desconto_adiciona_duas_colunas() {
        let lf = ler_transacoes(Path::new("dados/transacoes.csv")).expect("ok");
        let df = calcular_desconto(lf).expect("ok");

        assert!(df
            .get_column_names()
            .iter()
            .any(|nome| nome.as_str() == "desconto"));
        assert!(df
            .get_column_names()
            .iter()
            .any(|nome| nome.as_str() == "valor_final"));
    }
}
