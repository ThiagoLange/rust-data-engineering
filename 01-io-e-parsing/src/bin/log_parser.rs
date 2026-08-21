//! Módulo 01 — I/O e Parsing
//! README: "Exemplo prático 1: Parser de logs estruturados"
//!
//! Este exemplo junta os dois lados do módulo: `winnow` para extrair a
//! estrutura fixa de cada linha (`[timestamp] NIVEL mensagem key=value...`)
//! e `serde` para validar, de forma tipada, os campos `key=value` de linhas
//! que representam eventos conhecidos (aqui, `LOGIN`). É um padrão comum em
//! pipelines de ingestão de log: parsing "leve" e tolerante na entrada,
//! validação "forte" e tipada só na parte que o pipeline realmente precisa
//! entender.

use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use thiserror::Error;
use winnow::ascii::{alpha1, space1};
use winnow::combinator::delimited;
use winnow::token::take_till;
use winnow::Parser;

#[derive(Debug, Error)]
enum LogParseError {
    #[error("linha não começa com '[timestamp]': '{0}'")]
    CabecalhoAusente(String),
    #[error("erro de sintaxe no cabeçalho da linha: {0}")]
    SintaxeCabecalho(String),
}

/// Uma linha de log já estruturada: cabeçalho decomposto e o restante da
/// linha separado em mensagem livre + campos `chave=valor`.
#[derive(Debug, PartialEq)]
struct LogLinha {
    timestamp: String,
    nivel: String,
    mensagem: String,
    campos: BTreeMap<String, String>,
}

/// Consome `[timestamp] NIVEL ` do início da entrada usando winnow,
/// deixando em `entrada` só o restante (mensagem + campos). Como o parser
/// recebe `&mut &str`, cada combinator aplicado avança o ponteiro — ao
/// final da função, `entrada` já aponta pro que ainda falta processar.
fn parse_cabecalho<'e>(entrada: &mut &'e str) -> winnow::Result<(&'e str, &'e str)> {
    let timestamp = delimited('[', take_till(1.., ']'), ']').parse_next(entrada)?;
    space1.parse_next(entrada)?;
    let nivel = alpha1.parse_next(entrada)?;
    space1.parse_next(entrada)?;
    Ok((timestamp, nivel))
}

/// Separa o restante da linha (após o cabeçalho) em mensagem livre e campos
/// `chave=valor`. A regra do formato: a mensagem é composta pelas primeiras
/// palavras que NÃO contêm `=`; assim que aparece a primeira palavra com
/// `=`, ela e todas as seguintes são tratadas como campos estruturados.
///
/// Essa etapa é feita com `split_whitespace` em vez de mais combinators do
/// winnow de propósito: parser combinators brilham em estruturas fixas e
/// aninhadas, mas para "separar por espaço e classificar token a token" o
/// código padrão da std é mais direto e igualmente claro.
fn separar_mensagem_e_campos(resto: &str) -> (String, BTreeMap<String, String>) {
    let tokens: Vec<&str> = resto.split_whitespace().collect();
    let indice_primeiro_campo = tokens.iter().position(|token| token.contains('='));

    match indice_primeiro_campo {
        None => (tokens.join(" "), BTreeMap::new()),
        Some(indice) => {
            let mensagem = tokens[..indice].join(" ");
            let campos = tokens[indice..]
                .iter()
                .filter_map(|token| token.split_once('='))
                .map(|(chave, valor)| (chave.to_string(), valor.to_string()))
                .collect();
            (mensagem, campos)
        }
    }
}

/// Parseia uma linha de log completa, combinando o cabeçalho (winnow) com a
/// separação de mensagem/campos (acima).
fn parse_linha(linha: &str) -> Result<LogLinha, LogParseError> {
    if !linha.trim_start().starts_with('[') {
        return Err(LogParseError::CabecalhoAusente(linha.to_string()));
    }

    let mut entrada = linha;
    let (timestamp, nivel) = parse_cabecalho(&mut entrada)
        .map_err(|erro| LogParseError::SintaxeCabecalho(erro.to_string()))?;

    let (mensagem, campos) = separar_mensagem_e_campos(entrada);

    Ok(LogLinha {
        timestamp: timestamp.to_string(),
        nivel: nivel.to_string(),
        mensagem,
        campos,
    })
}

/// Schema tipado de um evento de login, usado para validar os campos de
/// linhas `LOGIN`. `user_id` é `u64` e `sucesso` é `bool` — se os campos
/// brutos não convertem para esses tipos, a validação falha com um erro
/// vindo direto do `serde`.
#[derive(Debug, Deserialize)]
struct EventoLogin {
    user_id: u64,
    sucesso: bool,
}

/// Converte um valor textual de campo (`"true"`, `"4821"`, `"senha_invalida"`)
/// para o tipo JSON mais específico que ele representa. Sem essa inferência,
/// todo campo viraria `Value::String` e `serde_json::from_value::<EventoLogin>`
/// nunca bateria com um campo `bool`/`u64`.
fn inferir_valor_json(bruto: &str) -> Value {
    if let Ok(booleano) = bruto.parse::<bool>() {
        return Value::Bool(booleano);
    }
    if let Ok(numero) = bruto.parse::<u64>() {
        return Value::Number(numero.into());
    }
    Value::String(bruto.to_string())
}

/// Tenta validar os campos brutos de uma linha `LOGIN` contra o schema
/// `EventoLogin`, reaproveitando o mesmo `serde_json::Value` dinâmico do
/// exemplo `02_serde_json_dinamico.rs` como ponte entre texto solto e
/// struct tipado.
fn validar_evento_login(
    campos: &BTreeMap<String, String>,
) -> Result<EventoLogin, serde_json::Error> {
    let objeto: Value = campos
        .iter()
        .map(|(chave, valor)| (chave.clone(), inferir_valor_json(valor)))
        .collect::<serde_json::Map<_, _>>()
        .into();

    serde_json::from_value(objeto)
}

fn main() -> anyhow::Result<()> {
    let caminho = Path::new("dados/logs/app.log");
    let conteudo = fs::read_to_string(caminho)?;

    let mut linhas_ok = 0;
    let mut linhas_com_erro = 0;

    for (numero, linha_bruta) in conteudo.lines().enumerate() {
        let numero_linha = numero + 1;
        match parse_linha(linha_bruta) {
            Ok(linha) => {
                linhas_ok += 1;
                println!(
                    "linha {numero_linha}: [{}] {} — {}",
                    linha.nivel, linha.timestamp, linha.mensagem
                );

                if linha.nivel == "LOGIN" {
                    match validar_evento_login(&linha.campos) {
                        Ok(evento) => println!(
                            "    evento de login válido: user_id={} sucesso={}",
                            evento.user_id, evento.sucesso
                        ),
                        Err(erro) => println!("    evento LOGIN com campos inválidos: {erro}"),
                    }
                }
            }
            Err(erro) => {
                linhas_com_erro += 1;
                println!("linha {numero_linha}: ERRO — {erro}");
            }
        }
    }

    println!("\nResumo: {linhas_ok} linhas válidas, {linhas_com_erro} linhas malformadas");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parseia_linha_bem_formada_com_campos() {
        let linha = "[2026-08-20T10:16:41Z] LOGIN tentativa de acesso user_id=4821 sucesso=true";
        let resultado = parse_linha(linha).unwrap();

        assert_eq!(resultado.timestamp, "2026-08-20T10:16:41Z");
        assert_eq!(resultado.nivel, "LOGIN");
        assert_eq!(resultado.mensagem, "tentativa de acesso");
        assert_eq!(resultado.campos.get("user_id"), Some(&"4821".to_string()));
        assert_eq!(resultado.campos.get("sucesso"), Some(&"true".to_string()));
    }

    #[test]
    fn parseia_linha_sem_campos_estruturados() {
        let linha = "[2026-08-20T10:19:30Z] INFO checkpoint concluido";
        let resultado = parse_linha(linha).unwrap();

        assert_eq!(resultado.mensagem, "checkpoint concluido");
        assert!(resultado.campos.is_empty());
    }

    #[test]
    fn linha_sem_colchete_de_abertura_falha() {
        let linha = "linha totalmente malformada sem colchetes nem nivel";
        assert!(matches!(
            parse_linha(linha),
            Err(LogParseError::CabecalhoAusente(_))
        ));
    }

    #[test]
    fn linha_sem_colchete_de_fechamento_falha() {
        let linha =
            "[2026-08-20T10:20:05Z LOGIN colchete de fechamento faltando user_id=100 sucesso=true";
        assert!(matches!(
            parse_linha(linha),
            Err(LogParseError::SintaxeCabecalho(_))
        ));
    }

    #[test]
    fn evento_login_com_campos_validos_e_aceito() {
        let mut campos = BTreeMap::new();
        campos.insert("user_id".to_string(), "4821".to_string());
        campos.insert("sucesso".to_string(), "true".to_string());

        let evento = validar_evento_login(&campos).unwrap();
        assert_eq!(evento.user_id, 4821);
        assert!(evento.sucesso);
    }

    #[test]
    fn evento_login_com_user_id_nao_numerico_e_rejeitado() {
        let mut campos = BTreeMap::new();
        campos.insert("user_id".to_string(), "abc".to_string());
        campos.insert("sucesso".to_string(), "true".to_string());

        assert!(validar_evento_login(&campos).is_err());
    }
}
