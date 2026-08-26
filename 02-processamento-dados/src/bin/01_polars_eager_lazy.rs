//! Módulo 02 — Processamento de Dados
//! README: seção "Conteúdo", item 1 — Polars eager vs lazy
//!
//! Polars oferece duas APIs para trabalhar com dados tabulares:
//!
//! - **Eager**: operações executam imediatamente, devolvendo um `DataFrame`
//!   concreto. É parecido com pandas: cada linha de código já lê/processa os
//!   dados na hora.
//! - **Lazy**: operações constroem um plano de execução (`LazyFrame`). Nada é
//!   executado até chamarmos `.collect()`. O motor lazy pode otimizar o plano
//!   inteiro antes de rodar — por exemplo, empurrar um filtro para dentro da
//!   leitura do CSV, evitando carregar linhas que seriam descartadas.
//!
//! A API lazy é a recomendada para pipelines de dados reais: ela permite
//! otimizações automáticas e consome menos memória em operações complexas.

use polars::prelude::*;
use std::path::Path;

/// Lê o CSV de forma *eager*: todo o arquivo vira um `DataFrame` na memória
/// e as operações são aplicadas imediatamente.
fn total_vendas_por_categoria_eager(caminho: &Path) -> anyhow::Result<DataFrame> {
    let mut df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some(caminho.to_path_buf()))?
        .finish()?;

    df = df
        .lazy()
        .group_by([col("categoria")])
        .agg([col("valor").sum().alias("total_vendas")])
        .collect()?;

    Ok(df)
}

/// Mesma operação, mas inteiramente *lazy*: o `LazyFrame` só descreve o que
/// deve ser feito. O `.collect()` no final é o ponto em que o plano otimizado
/// é executado.
fn total_vendas_por_categoria_lazy(caminho: &Path) -> anyhow::Result<DataFrame> {
    let lazy = LazyCsvReader::new(caminho.to_string_lossy().as_ref().into())
        .with_has_header(true)
        .finish()?;

    let resultado = lazy
        .group_by([col("categoria")])
        .agg([col("valor").sum().alias("total_vendas")])
        .collect()?;

    Ok(resultado)
}

/// Classifica transações em "alta", "média" ou "baixa" usando
/// `when/then/otherwise`. O mesmo código funciona tanto em eager quanto lazy
/// porque as expressões (`Expr`) são avaliadas pelo motor do Polars.
fn classificar_transacoes(lazy: LazyFrame) -> anyhow::Result<DataFrame> {
    lazy.with_columns([when(col("valor").gt(lit(500.0)))
        .then(lit("alta"))
        .when(col("valor").gt(lit(100.0)))
        .then(lit("media"))
        .otherwise(lit("baixa"))
        .alias("faixa")])
        .collect()
        .map_err(Into::into)
}

fn main() -> anyhow::Result<()> {
    let caminho = Path::new("dados/transacoes.csv");

    println!("=== Eager ===");
    let df_eager = total_vendas_por_categoria_eager(caminho)?;
    println!("{df_eager}");

    println!("\n=== Lazy ===");
    let df_lazy = total_vendas_por_categoria_lazy(caminho)?;
    println!("{df_lazy}");

    println!("\n=== Plano de execução lazy ===");
    let lazy = LazyCsvReader::new(caminho.to_string_lossy().as_ref().into())
        .with_has_header(true)
        .finish()?
        .group_by([col("categoria")])
        .agg([col("valor").sum().alias("total_vendas")]);
    println!("{}", lazy.explain(false)?);

    println!("\n=== Classificação de transações ===");
    let lazy = LazyCsvReader::new(caminho.to_string_lossy().as_ref().into())
        .with_has_header(true)
        .finish()?;
    let df_classificado = classificar_transacoes(lazy)?;
    println!("{}", df_classificado.head(Some(10)));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eager_e_lazy_produzem_mesmo_resultado() {
        let caminho = Path::new("dados/transacoes.csv");
        let eager = total_vendas_por_categoria_eager(caminho).expect("ok");
        let lazy = total_vendas_por_categoria_lazy(caminho).expect("ok");

        assert_eq!(eager.shape(), lazy.shape());
        assert!(eager["categoria"]
            .str()
            .unwrap()
            .iter()
            .any(|opt| opt == Some("eletronicos")));
    }

    #[test]
    fn classificacao_cria_coluna_faixa() {
        let caminho = Path::new("dados/transacoes.csv");
        let lazy = LazyCsvReader::new(caminho.to_string_lossy().as_ref().into())
            .with_has_header(true)
            .finish()
            .expect("ok");
        let df = classificar_transacoes(lazy).expect("ok");

        assert!(df
            .get_column_names()
            .iter()
            .any(|nome| nome.as_str() == "faixa"));
        let categorias = df["faixa"].unique().expect("ok");
        let valores: Vec<_> = categorias.str().unwrap().iter().flatten().collect();
        assert!(valores.contains(&"alta"));
        assert!(valores.contains(&"media"));
        assert!(valores.contains(&"baixa"));
    }
}
