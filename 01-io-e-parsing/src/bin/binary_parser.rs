//! Módulo 01 — I/O e Parsing
//! README: "Exemplo prático 2: Parser binário com winnow"
//!
//! Parsear bytes brutos "na unha" em Rust significa indexar o slice
//! manualmente, converter fatias de 4 bytes para números com
//! `u32::from_le_bytes`, e controlar um cursor (`offset`) que avança a cada
//! campo lido — um erro de contagem de bytes vira um bug silencioso (lê o
//! campo errado) em vez de um erro claro. `winnow` resolve isso oferecendo
//! parsers prontos para tipos binários (`le_u32`, `le_f32`, ...) que já
//! sabem quantos bytes consumir e avançam o cursor sozinhos — o mesmo
//! `&mut &[u8]` usado nos exemplos de texto deste módulo, só que operando
//! sobre bytes em vez de caracteres.
//!
//! Formato binário deste exemplo (ver `dados/binario/registros.bin`,
//! gerado com um script Python só para criar o arquivo de teste — o parser
//! em si é 100% Rust):
//!
//! ```text
//! cabeçalho (9 bytes):
//!   magic:          4 bytes  — literal b"RDEB"
//!   versao:         1 byte   — u8
//!   num_registros:  4 bytes  — u32 little-endian
//!
//! registro (9 bytes cada, repetido num_registros vezes):
//!   id:     4 bytes — u32 little-endian
//!   valor:  4 bytes — f32 little-endian
//!   ativo:  1 byte  — 0 = false, 1 = true
//! ```

use std::fs;
use std::path::Path;
use thiserror::Error;
use winnow::binary::{le_f32, le_u32, u8 as le_u8};
use winnow::token::literal;
use winnow::Parser;

const MAGIC: &[u8] = b"RDEB";

#[derive(Debug, Error)]
enum BinaryParseError {
    #[error("erro de sintaxe binária: {0}")]
    Sintaxe(String),
    #[error("byte de flag 'ativo' inválido: esperava 0 ou 1, encontrou {0}")]
    AtivoInvalido(u8),
}

#[derive(Debug, PartialEq)]
struct Cabecalho {
    versao: u8,
    num_registros: u32,
}

#[derive(Debug, PartialEq)]
struct Registro {
    id: u32,
    valor: f32,
    ativo: bool,
}

/// Parseia o cabeçalho fixo do formato: magic, versão e contagem de
/// registros. `literal(MAGIC)` falha imediatamente se os primeiros 4 bytes
/// não forem exatamente `b"RDEB"` — a forma do winnow de validar uma
/// assinatura de arquivo sem código manual de comparação de slices.
fn parse_cabecalho(entrada: &mut &[u8]) -> winnow::Result<Cabecalho> {
    literal(MAGIC).parse_next(entrada)?;
    let versao = le_u8.parse_next(entrada)?;
    let num_registros = le_u32.parse_next(entrada)?;
    Ok(Cabecalho {
        versao,
        num_registros,
    })
}

/// Parseia um único registro de tamanho fixo (9 bytes). Cada chamada de
/// parser (`le_u32`, `le_f32`, `le_u8`) consome exatamente os bytes do seu
/// tipo e avança `entrada` — não há como, sem querer, ler o campo seguinte
/// deslocado por um byte, erro clássico de parsing binário manual.
fn parse_registro(entrada: &mut &[u8]) -> Result<Registro, BinaryParseError> {
    let id = le_u32.parse_next(entrada).map_err(
        |erro: winnow::error::ErrMode<winnow::error::ContextError>| {
            BinaryParseError::Sintaxe(erro.to_string())
        },
    )?;
    let valor = le_f32.parse_next(entrada).map_err(
        |erro: winnow::error::ErrMode<winnow::error::ContextError>| {
            BinaryParseError::Sintaxe(erro.to_string())
        },
    )?;
    let ativo_bruto = le_u8.parse_next(entrada).map_err(
        |erro: winnow::error::ErrMode<winnow::error::ContextError>| {
            BinaryParseError::Sintaxe(erro.to_string())
        },
    )?;

    let ativo = match ativo_bruto {
        0 => false,
        1 => true,
        outro => return Err(BinaryParseError::AtivoInvalido(outro)),
    };

    Ok(Registro { id, valor, ativo })
}

/// Parseia o arquivo inteiro: cabeçalho seguido de `num_registros`
/// registros. O laço `for _ in 0..num_registros` é possível porque, ao
/// contrário de um formato terminado por delimitador, este formato diz
/// antecipadamente quantos registros existem — outra vantagem de formatos
/// binários bem desenhados sobre parsing "até o fim do arquivo".
fn parse_arquivo(bytes: &[u8]) -> Result<(Cabecalho, Vec<Registro>), BinaryParseError> {
    let mut entrada = bytes;

    let cabecalho = parse_cabecalho(&mut entrada)
        .map_err(|erro| BinaryParseError::Sintaxe(erro.to_string()))?;

    let mut registros = Vec::with_capacity(cabecalho.num_registros as usize);
    for _ in 0..cabecalho.num_registros {
        registros.push(parse_registro(&mut entrada)?);
    }

    Ok((cabecalho, registros))
}

fn main() -> anyhow::Result<()> {
    let caminho = Path::new("dados/binario/registros.bin");
    let bytes = fs::read(caminho)?;

    let (cabecalho, registros) = parse_arquivo(&bytes)?;

    println!(
        "Cabeçalho: versão {}, {} registros",
        cabecalho.versao, cabecalho.num_registros
    );
    for registro in &registros {
        println!("  {registro:?}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Monta bytes crus de um arquivo válido com os registros passados,
    /// para não depender do arquivo em disco nos testes unitários.
    fn montar_arquivo(registros: &[(u32, f32, u8)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MAGIC);
        bytes.push(1); // versao
        bytes.extend_from_slice(&(registros.len() as u32).to_le_bytes());
        for (id, valor, ativo) in registros {
            bytes.extend_from_slice(&id.to_le_bytes());
            bytes.extend_from_slice(&valor.to_le_bytes());
            bytes.push(*ativo);
        }
        bytes
    }

    #[test]
    fn parseia_arquivo_com_dois_registros() {
        let bytes = montar_arquivo(&[(1, 349.90, 1), (2, 89.50, 0)]);
        let (cabecalho, registros) = parse_arquivo(&bytes).unwrap();

        assert_eq!(
            cabecalho,
            Cabecalho {
                versao: 1,
                num_registros: 2
            }
        );
        assert_eq!(registros.len(), 2);
        assert_eq!(registros[0].id, 1);
        assert!(registros[0].ativo);
        assert!(!registros[1].ativo);
    }

    #[test]
    fn magic_invalido_falha() {
        let mut bytes = montar_arquivo(&[(1, 1.0, 1)]);
        bytes[0] = b'X'; // corrompe a assinatura
        assert!(parse_arquivo(&bytes).is_err());
    }

    #[test]
    fn byte_ativo_invalido_falha() {
        let mut bytes = montar_arquivo(&[(1, 1.0, 1)]);
        let ultimo = bytes.len() - 1;
        bytes[ultimo] = 7; // nem 0 nem 1
        let resultado = parse_arquivo(&bytes);
        assert!(matches!(resultado, Err(BinaryParseError::AtivoInvalido(7))));
    }

    #[test]
    fn arquivo_truncado_no_meio_de_um_registro_falha() {
        let mut bytes = montar_arquivo(&[(1, 1.0, 1), (2, 2.0, 1)]);
        bytes.truncate(bytes.len() - 3); // corta o último registro pela metade
        assert!(parse_arquivo(&bytes).is_err());
    }

    #[test]
    fn arquivo_de_disco_e_valido() {
        let bytes = fs::read("dados/binario/registros.bin").unwrap();
        let (cabecalho, registros) = parse_arquivo(&bytes).unwrap();
        assert_eq!(cabecalho.num_registros as usize, registros.len());
    }
}
