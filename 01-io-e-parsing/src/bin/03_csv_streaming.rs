//! Módulo 01 — I/O e Parsing
//! README: seção "Conteúdo", item 3 — csv crate (leitura/escrita streaming)
//!
//! A palavra-chave aqui é *streaming*: o crate `csv` não carrega o arquivo
//! inteiro em memória para depois processar — ele expõe um iterador que lê e
//! decodifica uma linha por vez, sob demanda. Para um arquivo de 50 GB isso
//! é a diferença entre processar com uso constante de memória (poucos MB) ou
//! estourar a RAM da máquina. Combinado com `serde`, cada linha já sai
//! deserializada direto para um struct tipado — sem parsing manual de
//! posição de coluna.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Schema esperado de cada linha do CSV de produtos. Note que `preco` é
/// `f64` — se uma linha tiver um valor não numérico nessa coluna, o `csv`
/// crate retorna `Err` para aquela linha específica em vez de interromper a
/// leitura de todo o arquivo, permitindo tratar erro linha a linha.
#[derive(Debug, Deserialize, Serialize)]
struct Produto {
    id: u32,
    nome: String,
    preco: f64,
    em_estoque: bool,
}

/// Lê um CSV linha a linha via streaming, separando as linhas que
/// deserializam com sucesso das que falham (guardando o número da linha e o
/// motivo do erro).
///
/// `ReaderBuilder::from_path` abre o arquivo mas não lê nada ainda.
/// `.deserialize::<Produto>()` retorna um iterador — cada `.next()` lê só a
/// próxima linha do disco. É esse iterador que dá a garantia de streaming.
type LinhaInvalida = (usize, String);

fn processar_csv_streaming(caminho: &Path) -> anyhow::Result<(Vec<Produto>, Vec<LinhaInvalida>)> {
    let mut leitor = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(caminho)?;

    let mut validos = Vec::new();
    let mut invalidos = Vec::new();

    // `.records()` cru serviria para ler campo a campo sem tipar; aqui
    // usamos `.deserialize()` para já obter `Produto` — mas mantemos o
    // índice manualmente porque o iterador de erro não carrega o número da
    // linha original (a primeira linha de dados é a 2, pois a 1 é o header).
    for (indice, resultado) in leitor.deserialize::<Produto>().enumerate() {
        let numero_linha = indice + 2;
        match resultado {
            Ok(produto) => validos.push(produto),
            Err(erro) => invalidos.push((numero_linha, erro.to_string())),
        }
    }

    Ok((validos, invalidos))
}

/// Escreve produtos de volta para CSV, também em modo streaming: cada
/// `.serialize()` grava uma linha no `BufWriter` interno do `Writer`, sem
/// acumular o CSV inteiro como string na memória antes de gravar.
fn escrever_csv_streaming(produtos: &[Produto], caminho: &Path) -> anyhow::Result<()> {
    let mut escritor = csv::Writer::from_path(caminho)?;
    for produto in produtos {
        escritor.serialize(produto)?;
    }
    escritor.flush()?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let caminho_entrada = Path::new("dados/produtos.csv");
    let (validos, invalidos) = processar_csv_streaming(caminho_entrada)?;

    println!("Produtos válidos: {}", validos.len());
    for produto in &validos {
        println!("  {produto:?}");
    }

    println!("\nLinhas inválidas: {}", invalidos.len());
    for (linha, motivo) in &invalidos {
        println!("  linha {linha}: {motivo}");
    }

    let caminho_saida = Path::new("dados/produtos_validos.csv");
    escrever_csv_streaming(&validos, caminho_saida)?;
    println!("\nProdutos válidos gravados em {}", caminho_saida.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn escrever_temp(conteudo: &str) -> std::path::PathBuf {
        let caminho = std::env::temp_dir().join(format!(
            "csv_streaming_teste_{}.csv",
            std::process::id().wrapping_add(rand_seed())
        ));
        let mut arquivo = std::fs::File::create(&caminho).unwrap();
        arquivo.write_all(conteudo.as_bytes()).unwrap();
        caminho
    }

    // Pequeno gerador de "aleatoriedade" só para variar o nome do arquivo
    // temporário entre execuções de teste em paralelo, sem trazer uma dep
    // extra (rand) só para isso.
    fn rand_seed() -> u32 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    }

    #[test]
    fn linhas_validas_sao_deserializadas() {
        let caminho = escrever_temp("id,nome,preco,em_estoque\n1,Caneta,2.50,true\n");
        let (validos, invalidos) = processar_csv_streaming(&caminho).unwrap();
        assert_eq!(validos.len(), 1);
        assert!(invalidos.is_empty());
        std::fs::remove_file(caminho).unwrap();
    }

    #[test]
    fn linha_com_preco_invalido_vai_para_invalidos() {
        let caminho = escrever_temp("id,nome,preco,em_estoque\n1,Caneta,abc,true\n");
        let (validos, invalidos) = processar_csv_streaming(&caminho).unwrap();
        assert!(validos.is_empty());
        assert_eq!(invalidos.len(), 1);
        assert_eq!(invalidos[0].0, 2);
        std::fs::remove_file(caminho).unwrap();
    }

    #[test]
    fn escreve_e_le_de_volta_preservando_dados() {
        let produtos = vec![
            Produto {
                id: 1,
                nome: "Caneta".to_string(),
                preco: 2.5,
                em_estoque: true,
            },
            Produto {
                id: 2,
                nome: "Lápis".to_string(),
                preco: 1.2,
                em_estoque: false,
            },
        ];
        let caminho = std::env::temp_dir().join(format!("csv_saida_{}.csv", rand_seed()));
        escrever_csv_streaming(&produtos, &caminho).unwrap();

        let (validos, invalidos) = processar_csv_streaming(&caminho).unwrap();
        assert_eq!(validos.len(), 2);
        assert!(invalidos.is_empty());
        std::fs::remove_file(caminho).unwrap();
    }
}
