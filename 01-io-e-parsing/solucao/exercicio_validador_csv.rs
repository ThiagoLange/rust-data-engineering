//! Módulo 01 — I/O e Parsing
//! Solução do exercício descrito no README ("Escreva um parser que valida
//! um arquivo CSV contra um schema declarado...").
//!
//! Diferente do `03_csv_streaming.rs` (que usa `serde` para deserializar
//! direto para um struct fixo), aqui o schema é *declarado em tempo de
//! execução* via linha de comando — o programa não sabe em tempo de
//! compilação quais colunas o CSV vai ter. Por isso a validação é feita
//! campo a campo, comparando o valor bruto (`&str`) contra o tipo esperado
//! declarado no schema, em vez de deixar o `serde` tentar (e falhar) a
//! conversão automaticamente.

use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

/// Tipos de coluna suportados pelo schema declarado via `--schema`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TipoColuna {
    Texto,
    Inteiro,
    Decimal,
    Booleano,
}

impl fmt::Display for TipoColuna {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nome = match self {
            TipoColuna::Texto => "texto",
            TipoColuna::Inteiro => "inteiro",
            TipoColuna::Decimal => "decimal",
            TipoColuna::Booleano => "booleano",
        };
        write!(f, "{nome}")
    }
}

#[derive(Debug, Clone)]
struct ColunaSchema {
    nome: String,
    tipo: TipoColuna,
    obrigatorio: bool,
}

#[derive(Debug, Error)]
enum SchemaError {
    #[error("declaração de coluna inválida: '{0}' (esperado nome:tipo:obrigatorio)")]
    FormatoInvalido(String),
    #[error("tipo de coluna desconhecido: '{0}' (use texto, inteiro, decimal ou booleano)")]
    TipoDesconhecido(String),
    #[error("coluna '{0}' declarada no schema não existe no cabeçalho do CSV")]
    ColunaAusenteNoCsv(String),
}

/// Schema de exemplo, usado quando `--schema` não é informado — bate com
/// as colunas de `dados/produtos.csv`.
const SCHEMA_PADRAO: &str =
    "id:inteiro:true,nome:texto:true,preco:decimal:true,em_estoque:booleano:false";

/// Parseia a string de schema recebida via `--schema`, no formato
/// `coluna:tipo:obrigatorio,coluna2:tipo2:obrigatorio2,...`.
fn parsear_schema(especificacao: &str) -> Result<Vec<ColunaSchema>, SchemaError> {
    especificacao
        .split(',')
        .map(str::trim)
        .filter(|parte| !parte.is_empty())
        .map(parsear_coluna_schema)
        .collect()
}

fn parsear_coluna_schema(declaracao: &str) -> Result<ColunaSchema, SchemaError> {
    let partes: Vec<&str> = declaracao.split(':').collect();
    let [nome, tipo_bruto, obrigatorio_bruto] = partes[..] else {
        return Err(SchemaError::FormatoInvalido(declaracao.to_string()));
    };

    let tipo = match tipo_bruto {
        "texto" => TipoColuna::Texto,
        "inteiro" => TipoColuna::Inteiro,
        "decimal" => TipoColuna::Decimal,
        "booleano" => TipoColuna::Booleano,
        outro => return Err(SchemaError::TipoDesconhecido(outro.to_string())),
    };

    let obrigatorio = obrigatorio_bruto.parse::<bool>().unwrap_or(false);

    Ok(ColunaSchema {
        nome: nome.to_string(),
        tipo,
        obrigatorio,
    })
}

/// Uma linha inválida do CSV, com o número da linha (contando o cabeçalho
/// como linha 1, como a maioria dos editores mostra) e o motivo.
#[derive(Debug, PartialEq)]
struct LinhaInvalida {
    linha: usize,
    motivo: String,
}

/// Confere se um valor bruto (texto) é compatível com o tipo declarado.
/// Campo vazio é tratado à parte por `validar_linha` (depende de
/// `obrigatorio`), então aqui uma string vazia sempre "passa" — ela só se
/// torna erro se a coluna for obrigatória.
fn valor_bate_com_tipo(tipo: TipoColuna, bruto: &str) -> bool {
    if bruto.is_empty() {
        return true;
    }
    match tipo {
        TipoColuna::Texto => true,
        TipoColuna::Inteiro => bruto.parse::<i64>().is_ok(),
        TipoColuna::Decimal => bruto.parse::<f64>().is_ok(),
        TipoColuna::Booleano => bruto.parse::<bool>().is_ok(),
    }
}

/// Valida uma linha (já decomposta em mapa coluna -> valor bruto) contra o
/// schema, retornando o primeiro motivo de invalidez encontrado, se houver.
/// Retornar no primeiro erro (em vez de acumular todos os erros da linha) é
/// uma escolha deliberada de simplicidade — suficiente para o relatório
/// pedido no exercício.
fn validar_linha(schema: &[ColunaSchema], campos: &BTreeMap<String, String>) -> Option<String> {
    for coluna in schema {
        let valor = match campos.get(&coluna.nome) {
            Some(valor) => valor,
            None => return Some(format!("campo '{}' ausente na linha", coluna.nome)),
        };

        if coluna.obrigatorio && valor.trim().is_empty() {
            return Some(format!(
                "campo obrigatório '{}' está em branco",
                coluna.nome
            ));
        }

        if !valor_bate_com_tipo(coluna.tipo, valor) {
            return Some(format!(
                "campo '{}' com valor '{}' não é do tipo {}",
                coluna.nome, valor, coluna.tipo
            ));
        }
    }
    None
}

/// Lê o CSV em modo streaming (mesma técnica de `03_csv_streaming.rs`, mas
/// sem `serde`, já que as colunas só são conhecidas em tempo de execução) e
/// valida cada linha contra o schema, coletando as inválidas.
fn validar_csv(
    caminho: &PathBuf,
    schema: &[ColunaSchema],
) -> anyhow::Result<(usize, Vec<LinhaInvalida>)> {
    // `flexible(true)`: por padrão o crate `csv` já rejeita a leitura
    // inteira se uma linha tiver número de campos diferente do header. Como
    // este exercício quer reportar isso como mais uma linha inválida no
    // relatório (não abortar o processamento), desligamos essa checagem
    // automática e fazemos a nossa própria logo abaixo.
    let mut leitor = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(caminho)?;

    let cabecalho: Vec<String> = leitor.headers()?.iter().map(str::to_string).collect();
    for coluna in schema {
        if !cabecalho.contains(&coluna.nome) {
            return Err(SchemaError::ColunaAusenteNoCsv(coluna.nome.clone()).into());
        }
    }

    let mut validas = 0;
    let mut invalidas = Vec::new();

    for (indice, resultado) in leitor.records().enumerate() {
        let numero_linha = indice + 2; // +1 pelo header, +1 porque indice começa em 0
        let registro = resultado?;

        if registro.len() != cabecalho.len() {
            invalidas.push(LinhaInvalida {
                linha: numero_linha,
                motivo: format!(
                    "número de campos incorreto: esperado {}, encontrado {}",
                    cabecalho.len(),
                    registro.len()
                ),
            });
            continue;
        }

        let campos: BTreeMap<String, String> = cabecalho
            .iter()
            .cloned()
            .zip(registro.iter().map(str::to_string))
            .collect();

        match validar_linha(schema, &campos) {
            Some(motivo) => invalidas.push(LinhaInvalida {
                linha: numero_linha,
                motivo,
            }),
            None => validas += 1,
        }
    }

    Ok((validas, invalidas))
}

struct Argumentos {
    entrada: PathBuf,
    schema: String,
}

/// Parsing manual de argumentos, no mesmo estilo do exercício do módulo 00
/// (`06_normalizador_csv.rs`): evita trazer `clap` só para dois parâmetros
/// opcionais.
fn parsear_argumentos(args: &[String]) -> anyhow::Result<Argumentos> {
    let mut entrada = PathBuf::from("dados/produtos.csv");
    let mut schema = SCHEMA_PADRAO.to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                let valor = args
                    .get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--input requer um valor"))?;
                entrada = PathBuf::from(valor);
                i += 2;
            }
            "--schema" => {
                let valor = args
                    .get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--schema requer um valor"))?;
                schema = valor.clone();
                i += 2;
            }
            outro => anyhow::bail!("argumento desconhecido: '{outro}'"),
        }
    }

    Ok(Argumentos { entrada, schema })
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let argumentos = parsear_argumentos(&args)?;
    let schema = parsear_schema(&argumentos.schema)?;

    println!(
        "Validando '{}' contra schema:",
        argumentos.entrada.display()
    );
    for coluna in &schema {
        let obrigatorio = if coluna.obrigatorio {
            "obrigatório"
        } else {
            "opcional"
        };
        println!("  {} : {} ({obrigatorio})", coluna.nome, coluna.tipo);
    }
    println!();

    let (validas, invalidas) = validar_csv(&argumentos.entrada, &schema)?;

    println!("Relatório de validação");
    println!("=======================");
    println!("Linhas válidas: {validas}");
    println!("Linhas inválidas: {}", invalidas.len());
    for linha_invalida in &invalidas {
        println!(
            "  linha {}: {}",
            linha_invalida.linha, linha_invalida.motivo
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema_produtos() -> Vec<ColunaSchema> {
        parsear_schema(SCHEMA_PADRAO).unwrap()
    }

    fn escrever_temp(conteudo: &str) -> PathBuf {
        use std::io::Write;
        use std::time::{SystemTime, UNIX_EPOCH};
        let sufixo = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let caminho = std::env::temp_dir().join(format!("validador_csv_teste_{sufixo}.csv"));
        let mut arquivo = std::fs::File::create(&caminho).unwrap();
        arquivo.write_all(conteudo.as_bytes()).unwrap();
        caminho
    }

    #[test]
    fn parseia_schema_com_multiplas_colunas() {
        let schema = schema_produtos();
        assert_eq!(schema.len(), 4);
        assert_eq!(schema[0].nome, "id");
        assert_eq!(schema[0].tipo, TipoColuna::Inteiro);
        assert!(schema[0].obrigatorio);
        assert!(!schema[3].obrigatorio);
    }

    #[test]
    fn linha_totalmente_valida_nao_gera_erro() {
        let caminho = escrever_temp("id,nome,preco,em_estoque\n1,Caneta,2.50,true\n");
        let (validas, invalidas) = validar_csv(&caminho, &schema_produtos()).unwrap();
        assert_eq!(validas, 1);
        assert!(invalidas.is_empty());
        std::fs::remove_file(caminho).unwrap();
    }

    #[test]
    fn campo_obrigatorio_em_branco_e_reportado() {
        let caminho = escrever_temp("id,nome,preco,em_estoque\n1,,2.50,true\n");
        let (validas, invalidas) = validar_csv(&caminho, &schema_produtos()).unwrap();
        assert_eq!(validas, 0);
        assert_eq!(invalidas.len(), 1);
        assert_eq!(invalidas[0].linha, 2);
        assert!(invalidas[0].motivo.contains("nome"));
    }

    #[test]
    fn campo_com_tipo_errado_e_reportado() {
        let caminho = escrever_temp("id,nome,preco,em_estoque\nsete,Caneta,2.50,true\n");
        let (_, invalidas) = validar_csv(&caminho, &schema_produtos()).unwrap();
        assert_eq!(invalidas.len(), 1);
        assert!(invalidas[0].motivo.contains("id"));
    }

    #[test]
    fn linha_com_numero_de_campos_errado_e_reportada() {
        let caminho = escrever_temp("id,nome,preco,em_estoque\n1,Caneta,2.50,true,extra\n");
        let (_, invalidas) = validar_csv(&caminho, &schema_produtos()).unwrap();
        assert_eq!(invalidas.len(), 1);
        assert!(invalidas[0].motivo.contains("número de campos"));
    }

    #[test]
    fn coluna_do_schema_ausente_no_csv_falha_antes_de_validar_linhas() {
        let caminho = escrever_temp("id,nome\n1,Caneta\n");
        let resultado = validar_csv(&caminho, &schema_produtos());
        assert!(resultado.is_err());
    }
}
