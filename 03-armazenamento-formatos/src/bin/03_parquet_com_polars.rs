//! Módulo 03 — Armazenamento e Formatos
//! README: seção "Conteúdo", item 3 — Parquet com Polars
//!
//! Polars lê e escreve Parquet nativamente. Este exemplo mostra como usar o
//! conhecimento do Módulo 02 para converter CSV em Parquet e aplicar filtros
//! lazy com predicate pushdown.

use anyhow::{Context, Result};
use polars::prelude::*;
use std::fs::File;
use std::path::Path;

fn csv_para_parquet(entrada: &Path, saida: &Path) -> Result<()> {
    let mut df = LazyCsvReader::new(entrada.to_string_lossy().as_ref().into())
        .with_has_header(true)
        .finish()?
        .collect()?;

    let mut file = File::create(saida).with_context(|| format!("criando {}", saida.display()))?;
    ParquetWriter::new(&mut file)
        .finish(&mut df)
        .map_err(|e| anyhow::anyhow!("escrevendo {}: {e}", saida.display()))?;
    Ok(())
}

fn ler_e_filtrar(caminho: &Path, regiao: &str) -> Result<DataFrame> {
    LazyFrame::scan_parquet(
        caminho.to_string_lossy().as_ref().into(),
        ScanArgsParquet::default(),
    )?
    .filter(col("regiao").eq(lit(regiao)))
    .collect()
    .map_err(Into::into)
}

fn main() -> Result<()> {
    let entrada = Path::new("dados/vendas.csv");
    let saida = Path::new("dados/saida/vendas_polars.parquet");
    let saida_parent = saida
        .parent()
        .ok_or_else(|| anyhow::anyhow!("caminho sem parent: {}", saida.display()))?;
    std::fs::create_dir_all(saida_parent).context("criando diretório de saída")?;

    csv_para_parquet(entrada, saida)?;
    println!("Parquet escrito em: {}", saida.display());

    let regioes = ["Sudeste", "Sul", "Norte", "Nordeste", "Centro-Oeste"];
    for regiao in regioes {
        let df = ler_e_filtrar(saida, regiao)?;
        println!("{regiao}: {} linhas", df.height());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn filtro_reduz_numero_de_linhas() {
        let entrada = Path::new("dados/vendas.csv");
        let saida = env::temp_dir().join("vendas_polars_test.parquet");
        csv_para_parquet(entrada, &saida).expect("ok");

        let total = LazyFrame::scan_parquet(
            saida.to_string_lossy().as_ref().into(),
            ScanArgsParquet::default(),
        )
        .expect("ok")
        .collect()
        .expect("ok")
        .height();

        let sudeste = ler_e_filtrar(&saida, "Sudeste").expect("ok").height();
        assert!(sudeste > 0);
        assert!(sudeste < total);

        std::fs::remove_file(saida).unwrap();
    }
}
