// README: 00-fundamentos § "Exemplo prático"
//
// Este é o exemplo principal do módulo: um CLI que lê um CSV
// (`id,valor`), normaliza a coluna `valor` (min-max, igual ao exemplo 02),
// e escreve o resultado — comparando o tempo de execução single-thread vs
// paralelo com rayon, exatamente como pede o README.
//
// Ele junta os quatro conceitos anteriores deste módulo:
// - ownership/borrow (01) pra passar os registros entre as etapas sem cópias
//   desnecessárias
// - traits/generics (02) — aqui simplificado, sem trait, pra manter o
//   exemplo direto ao ponto do CLI
// - error handling (03) — `thiserror` pro parser de linha, `anyhow` no CLI
// - concorrência (04/05) — `rayon` pra paralelizar a etapa CPU-bound
//
// Uso:
//   cargo run --release --bin 06_normalizador_csv -- --input dados/vendas.csv --output resultado.csv

use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use std::fs;
use std::time::Instant;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
enum LinhaInvalida {
    #[error("linha {numero} tem {encontrado} campos, esperava 2 (id,valor)")]
    NumeroDeCamposErrado { numero: usize, encontrado: usize },
    #[error("linha {numero}: valor {valor_bruto:?} não é um número válido")]
    ValorInvalido { numero: usize, valor_bruto: String },
}

#[derive(Debug, Clone, PartialEq)]
struct Registro {
    id: String,
    valor: f64,
}

/// Argumentos de linha de comando, já validados.
/// Parsing manual (sem `clap`) porque só temos duas flags obrigatórias —
/// adicionar uma dependência inteira de CLI parsing pra isso seria
/// exatamente o tipo de crate "por via das dúvidas" que o CLAUDE.md deste
/// repo pede pra evitar.
struct Argumentos {
    input: String,
    output: String,
}

fn parsear_argumentos(args: &[String]) -> Result<Argumentos> {
    let mut input = None;
    let mut output = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                input = args.get(i + 1).cloned();
                i += 2;
            }
            "--output" => {
                output = args.get(i + 1).cloned();
                i += 2;
            }
            outro => bail!("argumento desconhecido: {outro}"),
        }
    }

    Ok(Argumentos {
        input: input.context("faltou --input <caminho>")?,
        output: output.context("faltou --output <caminho>")?,
    })
}

/// Parseia o conteúdo inteiro do CSV (assumindo cabeçalho `id,valor` na
/// primeira linha) em uma lista de registros.
fn parsear_csv(conteudo: &str) -> Result<Vec<Registro>, LinhaInvalida> {
    conteudo
        .lines()
        .skip(1) // pula o cabeçalho
        .filter(|linha| !linha.trim().is_empty())
        .enumerate()
        .map(|(indice, linha)| {
            let numero = indice + 2; // +1 pelo índice base-0, +1 pelo cabeçalho
            let campos: Vec<&str> = linha.split(',').collect();
            if campos.len() != 2 {
                return Err(LinhaInvalida::NumeroDeCamposErrado {
                    numero,
                    encontrado: campos.len(),
                });
            }
            let valor =
                campos[1]
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| LinhaInvalida::ValorInvalido {
                        numero,
                        valor_bruto: campos[1].to_string(),
                    })?;
            Ok(Registro {
                id: campos[0].trim().to_string(),
                valor,
            })
        })
        .collect()
}

/// Normalização min-max sequencial — um núcleo, um registro de cada vez.
fn normalizar_sequencial(registros: &[Registro]) -> Vec<f64> {
    let minimo = registros
        .iter()
        .map(|r| r.valor)
        .fold(f64::INFINITY, f64::min);
    let maximo = registros
        .iter()
        .map(|r| r.valor)
        .fold(f64::NEG_INFINITY, f64::max);
    let amplitude = maximo - minimo;

    registros
        .iter()
        .map(|r| {
            if amplitude.abs() < f64::EPSILON {
                0.0
            } else {
                (r.valor - minimo) / amplitude
            }
        })
        .collect()
}

/// Mesma normalização, mas com `rayon`: min/max via `.par_iter().reduce(...)`
/// e o mapeamento final via `.par_iter().map(...)`. Pra um dataset pequeno
/// como o deste módulo (milhares de linhas), a versão paralela pode até ser
/// mais lenta que a sequencial — o overhead de dividir trabalho entre
/// threads só compensa quando o volume de dados é grande o suficiente. Vale
/// rodar com `--release` e um CSV maior pra ver a diferença de verdade.
fn normalizar_paralelo(registros: &[Registro]) -> Vec<f64> {
    let minimo = registros
        .par_iter()
        .map(|r| r.valor)
        .reduce(|| f64::INFINITY, f64::min);
    let maximo = registros
        .par_iter()
        .map(|r| r.valor)
        .reduce(|| f64::NEG_INFINITY, f64::max);
    let amplitude = maximo - minimo;

    registros
        .par_iter()
        .map(|r| {
            if amplitude.abs() < f64::EPSILON {
                0.0
            } else {
                (r.valor - minimo) / amplitude
            }
        })
        .collect()
}

fn escrever_resultado(caminho: &str, registros: &[Registro], normalizados: &[f64]) -> Result<()> {
    let mut saida = String::from("id,valor,valor_normalizado\n");
    for (registro, valor_normalizado) in registros.iter().zip(normalizados) {
        saida.push_str(&format!(
            "{},{},{valor_normalizado:.6}\n",
            registro.id, registro.valor
        ));
    }
    fs::write(caminho, saida).with_context(|| format!("falha ao escrever {caminho}"))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let argumentos = parsear_argumentos(&args)?;

    let conteudo = fs::read_to_string(&argumentos.input)
        .with_context(|| format!("falha ao ler arquivo de entrada {}", argumentos.input))?;
    let registros = parsear_csv(&conteudo).context("falha ao parsear CSV de entrada")?;
    println!("Registros carregados: {}", registros.len());

    let inicio = Instant::now();
    let normalizados_seq = normalizar_sequencial(&registros);
    let duracao_seq = inicio.elapsed();

    let inicio = Instant::now();
    let normalizados_par = normalizar_paralelo(&registros);
    let duracao_par = inicio.elapsed();

    println!("Normalização sequencial: {duracao_seq:?}");
    println!("Normalização paralela (rayon): {duracao_par:?}");

    escrever_resultado(&argumentos.output, &registros, &normalizados_par)?;
    println!("Resultado escrito em {}", argumentos.output);

    debug_assert_eq!(normalizados_seq.len(), normalizados_par.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn csv_exemplo() -> &'static str {
        "id,valor\na,0\nb,50\nc,100\n"
    }

    #[test]
    fn parsear_csv_le_registros_validos() {
        let registros = parsear_csv(csv_exemplo()).expect("csv válido");
        assert_eq!(
            registros,
            vec![
                Registro {
                    id: "a".into(),
                    valor: 0.0
                },
                Registro {
                    id: "b".into(),
                    valor: 50.0
                },
                Registro {
                    id: "c".into(),
                    valor: 100.0
                },
            ]
        );
    }

    #[test]
    fn parsear_csv_reporta_numero_da_linha_no_erro() {
        let csv = "id,valor\na,10\nb,xis\n";
        let erro = parsear_csv(csv).unwrap_err();
        assert_eq!(
            erro,
            LinhaInvalida::ValorInvalido {
                numero: 3,
                valor_bruto: "xis".into()
            }
        );
    }

    #[test]
    fn normalizar_sequencial_e_paralelo_produzem_o_mesmo_resultado() {
        let registros = parsear_csv(csv_exemplo()).expect("csv válido");
        let seq = normalizar_sequencial(&registros);
        let par = normalizar_paralelo(&registros);
        assert_eq!(seq, par);
        assert_eq!(seq, vec![0.0, 0.5, 1.0]);
    }

    #[test]
    fn parsear_argumentos_exige_input_e_output() {
        let args = vec!["--input".to_string(), "a.csv".to_string()];
        assert!(parsear_argumentos(&args).is_err());

        let args = vec![
            "--input".to_string(),
            "a.csv".to_string(),
            "--output".to_string(),
            "b.csv".to_string(),
        ];
        let parsed = parsear_argumentos(&args).expect("args completos");
        assert_eq!(parsed.input, "a.csv");
        assert_eq!(parsed.output, "b.csv");
    }
}
