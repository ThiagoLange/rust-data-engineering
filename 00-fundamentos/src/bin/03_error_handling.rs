// README: 00-fundamentos § "Error handling"
//
// Rust não tem exceptions. Toda função que pode falhar retorna
// `Result<T, E>`: ou `Ok(valor)`, ou `Err(erro)`. O compilador OBRIGA você a
// lidar com o caso de erro de alguma forma (match, `?`, `.unwrap_or(...)`)
// — não tem como "esquecer" um try/catch e deixar a exceção estourar em
// produção sem ninguém saber que aquele caminho existia.
//
// Duas ferramentas, dois papéis (ver CLAUDE.md deste repo):
// - `thiserror`: pra bibliotecas/código reutilizável — define um enum de
//   erro específico e tipado, que quem chama pode inspecionar e tratar
//   diferente por variante.
// - `anyhow`: pra binários/prototipagem — um "Result<T, qualquer erro>"
//   com contexto encadeável, quando você só quer propagar o erro pra cima
//   e imprimir uma mensagem legível, sem modelar cada variante.

use anyhow::Context;
use thiserror::Error;

/// Enum de erro tipado — típico de uma função que seria parte de uma
/// biblioteca reutilizável (ex: um parser de linha de CSV usado por vários
/// binários deste módulo). Cada variante carrega os dados relevantes pro
/// erro específico.
#[derive(Debug, Error, PartialEq, Eq)]
enum ParseRegistroError {
    #[error("linha vazia não é um registro válido")]
    LinhaVazia,

    #[error("esperava 2 campos (produto,valor) mas encontrou {encontrado}")]
    NumeroDeCamposErrado { encontrado: usize },

    #[error("campo 'valor' não é um número válido: {valor_bruto:?}")]
    ValorInvalido { valor_bruto: String },
}

/// Parseia uma linha "produto,valor" em (produto, valor: f64).
/// Retorna nosso erro tipado — quem chama pode fazer `match` e decidir,
/// por exemplo, "linha vazia eu ignoro, valor inválido eu aborto o lote".
fn parsear_linha(linha: &str) -> Result<(String, f64), ParseRegistroError> {
    if linha.trim().is_empty() {
        return Err(ParseRegistroError::LinhaVazia);
    }

    let campos: Vec<&str> = linha.split(',').collect();
    if campos.len() != 2 {
        return Err(ParseRegistroError::NumeroDeCamposErrado {
            encontrado: campos.len(),
        });
    }

    // O operador `?` aqui faria propagação automática SE os tipos de erro
    // batessem. Como `parse::<f64>()` retorna `ParseFloatError` (tipo
    // diferente do nosso enum), convertemos explicitamente com `.map_err`.
    let valor = campos[1]
        .trim()
        .parse::<f64>()
        .map_err(|_| ParseRegistroError::ValorInvalido {
            valor_bruto: campos[1].to_string(),
        })?;

    Ok((campos[0].trim().to_string(), valor))
}

/// Processa várias linhas, usando `?` de verdade agora: como
/// `processar_lote` retorna o MESMO tipo de erro que `parsear_linha`, `?`
/// simplesmente propaga o `Err` pra cima na primeira linha inválida —
/// sem try/catch, sem exceção "escondida".
fn processar_lote(linhas: &[&str]) -> Result<Vec<(String, f64)>, ParseRegistroError> {
    let mut registros = Vec::with_capacity(linhas.len());
    for linha in linhas {
        registros.push(parsear_linha(linha)?);
    }
    Ok(registros)
}

/// No `main`, trocamos pra `anyhow::Result`: não nos importa mais QUAL
/// variante falhou, só queremos propagar com uma mensagem de contexto
/// legível pra quem está rodando o binário. `.context(...)` encadeia uma
/// mensagem amigável na frente do erro original.
fn main() -> anyhow::Result<()> {
    let linhas_validas = ["teclado,250.0", "monitor,900.0"];
    let registros = processar_lote(&linhas_validas).context("falha ao processar lote de vendas")?;
    println!("Registros processados: {registros:?}");

    // Exemplo de tratamento explícito via `match`, sem propagar: útil
    // quando cada variante de erro merece uma reação diferente.
    match parsear_linha("mouse,não-é-numero") {
        Ok(registro) => println!("Registro válido: {registro:?}"),
        Err(ParseRegistroError::ValorInvalido { valor_bruto }) => {
            println!("Ignorando registro com valor inválido: {valor_bruto:?}");
        }
        Err(outro_erro) => println!("Erro inesperado: {outro_erro}"),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsear_linha_valida_retorna_produto_e_valor() {
        let resultado = parsear_linha("teclado,250.0");
        assert_eq!(resultado, Ok(("teclado".to_string(), 250.0)));
    }

    #[test]
    fn parsear_linha_vazia_retorna_erro_especifico() {
        let resultado = parsear_linha("   ");
        assert_eq!(resultado, Err(ParseRegistroError::LinhaVazia));
    }

    #[test]
    fn parsear_linha_com_campos_a_mais_retorna_erro_especifico() {
        let resultado = parsear_linha("a,b,c");
        assert_eq!(
            resultado,
            Err(ParseRegistroError::NumeroDeCamposErrado { encontrado: 3 })
        );
    }

    #[test]
    fn parsear_linha_com_valor_nao_numerico_retorna_erro_especifico() {
        let resultado = parsear_linha("mouse,caro");
        assert_eq!(
            resultado,
            Err(ParseRegistroError::ValorInvalido {
                valor_bruto: "caro".to_string()
            })
        );
    }

    #[test]
    fn processar_lote_para_na_primeira_linha_invalida() {
        let linhas = ["ok,1.0", "quebrada"];
        let resultado = processar_lote(&linhas);
        assert_eq!(
            resultado,
            Err(ParseRegistroError::NumeroDeCamposErrado { encontrado: 1 })
        );
    }
}
