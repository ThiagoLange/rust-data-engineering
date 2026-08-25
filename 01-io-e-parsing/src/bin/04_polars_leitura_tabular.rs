//! Módulo 01 — I/O e Parsing
//! README: seção "Conteúdo", item 4 — Polars para leitura tabular
//!
//! `csv` (item anterior) e `serde_json` trabalham *linha a linha* ou
//! *registro a registro*: bons para validar e transformar dados um item por
//! vez. `Polars` pensa em *colunas*: um `DataFrame` guarda cada coluna como
//! um bloco contíguo de memória do mesmo tipo, o que permite operações
//! vetorizadas (somar uma coluna inteira, filtrar, agrupar) muito mais
//! rápidas do que iterar linha a linha em Rust puro. Isso é só uma
//! introdução ao `read_csv`/filtro/agregação — o Módulo 2 aprofunda em
//! LazyFrames, plano de execução e otimizações.
//!
//! Note o padrão "lazy": construímos um `LazyFrame` descrevendo as
//! operações desejadas (ler, filtrar, agrupar) e só quando chamamos
//! `.collect()` é que o Polars de fato executa — podendo antes otimizar o
//! plano inteiro (ex: aplicar o filtro durante a leitura, não depois).

use polars::prelude::*;
use std::path::Path;

/// Força a conversão da coluna `em_estoque` para booleano.
///
/// O CSV de exemplo tem `em_estoque` como `str` porque contém o valor
/// inválido "talvez". Sem esse cast, o filtro `eq(lit(true))` compararia
/// strings com booleanos de forma implícita — o resultado pode mudar entre
/// versões do Polars e confunde quem está aprendendo. Com `.cast(Boolean)`,
/// valores como "true"/"false" viram `true`/`false`, valores inválidos
/// viram `null` e são descartados pelo filtro.
fn para_bool(expr: Expr) -> Expr {
    expr.cast(DataType::Boolean)
}

/// Lê o CSV de produtos como um `LazyFrame`. `LazyCsvReader` não lê o
/// arquivo aqui — apenas registra a intenção de lê-lo, permitindo que o
/// Polars monte um plano de execução com o restante das operações
/// encadeadas depois (`.filter()`, `.select()`, etc) antes de rodar tudo de
/// uma vez com `.collect()`.
fn ler_produtos(caminho: &Path) -> anyhow::Result<LazyFrame> {
    let lazy = LazyCsvReader::new(caminho.to_string_lossy().as_ref().into())
        .with_has_header(true)
        // O CSV de exemplo tem uma linha com um campo extra (ver
        // 03_csv_streaming.rs); sem isso, o Polars rejeita a leitura
        // inteira em vez de só a linha problemática.
        .with_truncate_ragged_lines(true)
        .finish()?;
    Ok(lazy)
}

/// Filtra produtos em estoque com preço acima de um limite, e seleciona só
/// as colunas relevantes — tudo isso ainda de forma "preguiçosa" (lazy),
/// sem executar nada até o `.collect()` do chamador.
fn produtos_caros_em_estoque(lazy: LazyFrame, preco_minimo: f64) -> LazyFrame {
    lazy.filter(
        para_bool(col("em_estoque"))
            .eq(lit(true))
            .and(col("preco").gt(lit(preco_minimo))),
    )
    .select([col("id"), col("nome"), col("preco")])
}

fn main() -> anyhow::Result<()> {
    let caminho = Path::new("dados/produtos.csv");

    // O CSV de exemplo tem linhas propositalmente inválidas (ver
    // 03_csv_streaming.rs) — Polars, ao contrário do crate `csv` com
    // `serde`, não falha a leitura inteira por causa delas: campos que não
    // convertem para o tipo da coluna viram `null`, e linhas com campos a
    // mais são truncadas (graças a `with_truncate_ragged_lines` acima).
    // Isso é útil para exploração rápida, mas esconde erros que o
    // validador do exercício deste módulo (solucao/) reporta explicitamente.
    let lazy = ler_produtos(caminho)?;

    let df_completo = lazy.clone().collect()?;
    println!("DataFrame completo:\n{df_completo}\n");

    let df_filtrado = produtos_caros_em_estoque(lazy, 100.0).collect()?;
    println!("Produtos em estoque com preço > 100:\n{df_filtrado}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn escrever_temp(conteudo: &str) -> std::path::PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};
        let sufixo = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let caminho = std::env::temp_dir().join(format!("polars_teste_{sufixo}.csv"));
        let mut arquivo = std::fs::File::create(&caminho).unwrap();
        arquivo.write_all(conteudo.as_bytes()).unwrap();
        caminho
    }

    #[test]
    fn le_csv_com_numero_esperado_de_linhas() {
        let caminho = escrever_temp("id,nome,preco,em_estoque\n1,A,10.0,true\n2,B,20.0,false\n");
        let df = ler_produtos(&caminho).unwrap().collect().unwrap();
        assert_eq!(df.height(), 2);
        std::fs::remove_file(caminho).unwrap();
    }

    #[test]
    fn filtro_retorna_apenas_produtos_em_estoque_e_caros() {
        let caminho = escrever_temp(
            "id,nome,preco,em_estoque\n1,Barato,10.0,true\n2,Caro,500.0,true\n3,ForaEstoque,900.0,false\n",
        );
        let lazy = ler_produtos(&caminho).unwrap();
        let df = produtos_caros_em_estoque(lazy, 100.0).collect().unwrap();
        assert_eq!(df.height(), 1);
        std::fs::remove_file(caminho).unwrap();
    }
}
