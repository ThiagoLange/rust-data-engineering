//! Módulo 01 — I/O e Parsing
//! README: seção "Conteúdo", item 5 — parsing com winnow (parser combinators)
//!
//! `serde`/`csv`/`polars` resolvem formatos *padronizados* (JSON, CSV).
//! Mas dados de engenharia frequentemente vêm em formatos proprietários sem
//! biblioteca pronta: logs customizados, protocolos binários internos,
//! configs com uma sintaxe própria. Escrever esse parsing "na unha" com
//! `split`/`find`/índices manuais é frágil e propenso a bugs sutis em casos
//! de borda. *Parser combinators* resolvem isso combinando parsers pequenos
//! e testáveis (um parser de "uma palavra", um de "um número", um de "até
//! o próximo `;`") em parsers maiores — cada peça pequena é fácil de
//! entender e testar isoladamente.
//!
//! Este exemplo introduz os combinators básicos do `winnow` parseando um
//! texto simples `chave=valor;chave2=valor2`. O exemplo prático completo do
//! módulo (`binary_parser.rs`) usa os mesmos conceitos para parsear bytes
//! binários, não texto.

use winnow::ascii::alphanumeric1;
use winnow::combinator::separated;
use winnow::combinator::separated_pair;
use winnow::token::take_till;
use winnow::Parser;
use winnow::Result as WinnowResult;

/// Parser de uma "chave": uma sequência de caracteres alfanuméricos ou `_`.
/// `winnow` opera sobre um `&mut &str` (a referência é avançada conforme o
/// parser consome caracteres) e retorna a fatia consumida.
fn chave<'entrada>(entrada: &mut &'entrada str) -> WinnowResult<&'entrada str> {
    // `alphanumeric1` sozinho não aceita `_`; para o propósito didático
    // deste exemplo, tratamos chaves como puramente alfanuméricas.
    alphanumeric1.parse_next(entrada)
}

/// Parser de um "valor": tudo até o próximo `;` ou até o fim da entrada.
/// `take_till` é um combinator de baixo nível — consome caracteres
/// enquanto o predicado for falso, sem interpretar o conteúdo.
fn valor<'entrada>(entrada: &mut &'entrada str) -> WinnowResult<&'entrada str> {
    take_till(0.., |c| c == ';').parse_next(entrada)
}

/// Combina `chave` e `valor` separados por `=`. `separated_pair` é um dos
/// combinators mais usados: recebe três parsers (esquerda, separador,
/// direita) e retorna uma tupla com os dois resultados relevantes,
/// descartando o separador.
fn par_chave_valor<'entrada>(
    entrada: &mut &'entrada str,
) -> WinnowResult<(&'entrada str, &'entrada str)> {
    separated_pair(chave, '=', valor).parse_next(entrada)
}

/// Combina vários pares `chave=valor` separados por `;` numa lista.
/// `separated` é o combinator de repetição: aplica o parser interno quantas
/// vezes conseguir, intercalando com o separador, e para quando não
/// encontra mais nenhum.
fn lista_de_pares<'entrada>(
    entrada: &mut &'entrada str,
) -> WinnowResult<Vec<(&'entrada str, &'entrada str)>> {
    separated(0.., par_chave_valor, ';').parse_next(entrada)
}

/// Função de conveniência que roda o parser completo sobre uma string e
/// converte o resultado para `anyhow::Result`, já que os combinators do
/// winnow retornam seu próprio tipo de erro (rico em contexto, mas não
/// diretamente compatível com `anyhow`).
fn parsear_config(texto: &str) -> anyhow::Result<Vec<(String, String)>> {
    let mut entrada = texto;
    let pares = lista_de_pares
        .parse_next(&mut entrada)
        .map_err(|erro| anyhow::anyhow!("falha ao parsear '{texto}': {erro}"))?;

    if !entrada.is_empty() {
        anyhow::bail!("sobrou entrada não consumida: '{entrada}'");
    }

    Ok(pares
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect())
}

fn main() -> anyhow::Result<()> {
    let linha = "ambiente=producao;versao=1.4.2;regiao=us-east";
    let pares = parsear_config(linha)?;

    println!("Linha original: {linha}");
    println!("Pares extraídos:");
    for (chave, valor) in &pares {
        println!("  {chave} = {valor}");
    }

    // Entrada malformada: falta o "=" no segundo par.
    let linha_invalida = "ambiente=producao;versao";
    match parsear_config(linha_invalida) {
        Ok(_) => println!("\n(inesperado: deveria ter falhado)"),
        Err(erro) => println!("\nEntrada inválida detectada corretamente: {erro}"),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parseia_um_unico_par() {
        let pares = parsear_config("chave=valor").unwrap();
        assert_eq!(pares, vec![("chave".to_string(), "valor".to_string())]);
    }

    #[test]
    fn parseia_varios_pares() {
        let pares = parsear_config("a=1;b=2;c=3").unwrap();
        assert_eq!(
            pares,
            vec![
                ("a".to_string(), "1".to_string()),
                ("b".to_string(), "2".to_string()),
                ("c".to_string(), "3".to_string()),
            ]
        );
    }

    #[test]
    fn falha_quando_falta_o_sinal_de_igual() {
        let resultado = parsear_config("chave_sem_valor;outra=1");
        assert!(resultado.is_err());
    }

    #[test]
    fn entrada_vazia_retorna_lista_vazia() {
        let pares = parsear_config("").unwrap();
        assert!(pares.is_empty());
    }
}
