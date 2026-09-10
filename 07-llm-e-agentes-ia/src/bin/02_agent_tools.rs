//! Módulo 07 — LLM e Agentes
//! README: seção "Exemplo prático 2: Agente com tool calling"
//!
//! Agente com duas ferramentas: busca em dados locais e calculadora.
//! Demonstra function calling com validação via `schemars` e `serde_json`,
//! e encadeamento de resultados. Sem `rig` para manter build leve — padrão
//! manual equivalente, compatível com `async-openai` function calling.

use anyhow::{Context, Result};
use clap::Parser;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Parser, Debug)]
#[command(name = "agent_tools")]
struct Args {
    /// Pergunta para o agente
    #[arg(
        default_value = "Quantos registros tem a tabela vendas e qual a média de preco_unitario?"
    )]
    query: String,
}

// --- Ferramenta 1: busca em dados locais (simula DataFusion/SQL) ---

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct BuscaDadosArgs {
    /// Nome da tabela (ex: vendas)
    tabela: String,
    /// Coluna para agregação (ex: preco_unitario)
    coluna: Option<String>,
    /// Operação: count, avg, sum, max, min
    operacao: String,
}

fn ferramenta_busca_dados(args: BuscaDadosArgs) -> Result<String> {
    // Dados sintéticos de vendas (simula consulta SQL)
    // Em produção: `SELECT COUNT(*) FROM vendas`, `AVG(preco_unitario)` via DataFusion
    let total_registros = 1000;
    let media_preco = 245.67;
    let soma_preco = 245670.0;

    let resultado = match args.operacao.as_str() {
        "count" => serde_json::json!({"tabela": args.tabela, "count": total_registros}),
        "avg" => {
            serde_json::json!({"tabela": args.tabela, "coluna": args.coluna, "avg": media_preco})
        }
        "sum" => serde_json::json!({"sum": soma_preco}),
        _ => serde_json::json!({"erro": "operacao não suportada"}),
    };
    Ok(resultado.to_string())
}

// --- Ferramenta 2: calculadora ---

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CalculadoraArgs {
    /// Expressão aritmética (ex: "2 + 3 * 4")
    expressao: String,
}

fn ferramenta_calculadora(args: CalculadoraArgs) -> Result<String> {
    // Parser simples: usa `eval` via `meval` crate? Para demo, implementa manual
    // Suporta + - * / e parênteses com `eval` simples via `f64` parsing
    // Aqui simplificamos para `a + b` ou `a * b` para manter sem deps extras
    let expr = args.expressao.replace(' ', "");
    // Tenta avaliar com `eval` crate não disponível — faz parse manual simples
    // Para demo, usa `serde_json` para números e operações básicas
    let resultado = avaliar_expressao(&expr)?;
    Ok(serde_json::json!({"expressao": args.expressao, "resultado": resultado}).to_string())
}

fn avaliar_expressao(expr: &str) -> Result<f64> {
    // Parser recursivo simples para + - * / e parênteses
    // Usa `meval` se disponível, senão fallback para `eval` manual
    // Para manter sem deps, implementa via `f64` e `eval` com `std`
    // Aqui delegamos para `eval` via `js`? Simplificamos: usa `f64` parse com `split`
    // Suporta apenas expressões binárias simples para demo
    if expr.contains('+') {
        let parts: Vec<&str> = expr.split('+').collect();
        if parts.len() == 2 {
            let a: f64 = parts[0].parse().context("parse a")?;
            let b: f64 = parts[1].parse().context("parse b")?;
            return Ok(a + b);
        }
    }
    if expr.contains('*') {
        let parts: Vec<&str> = expr.split('*').collect();
        if parts.len() == 2 {
            let a: f64 = parts[0].parse().context("parse a")?;
            let b: f64 = parts[1].parse().context("parse b")?;
            return Ok(a * b);
        }
    }
    if expr.contains('/') {
        let parts: Vec<&str> = expr.split('/').collect();
        if parts.len() == 2 {
            let a: f64 = parts[0].parse().context("parse a")?;
            let b: f64 = parts[1].parse().context("parse b")?;
            return Ok(a / b);
        }
    }
    if expr.contains('-') && !expr.starts_with('-') {
        let parts: Vec<&str> = expr.split('-').collect();
        if parts.len() == 2 {
            let a: f64 = parts[0].parse().context("parse a")?;
            let b: f64 = parts[1].parse().context("parse b")?;
            return Ok(a - b);
        }
    }
    // Fallback: tenta parse como número puro
    expr.parse::<f64>().context("expressão não suportada")
}

// --- Agente ---

#[derive(Debug, Clone)]
struct ToolCall {
    tool: String,
    args: Value,
}

fn decidir_ferramentas(query: &str) -> Vec<ToolCall> {
    let q = query.to_lowercase();
    let mut calls = Vec::new();

    if q.contains("quantos")
        || q.contains("registros")
        || q.contains("média")
        || q.contains("media")
        || q.contains("count")
        || q.contains("avg")
    {
        // Decide se precisa de count ou avg
        if q.contains("média") || q.contains("media") || q.contains("avg") {
            calls.push(ToolCall {
                tool: "buscar_dados".to_string(),
                args: serde_json::json!({"tabela": "vendas", "coluna": "preco_unitario", "operacao": "avg"}),
            });
        }
        if q.contains("quantos") || q.contains("registros") || q.contains("count") {
            calls.push(ToolCall {
                tool: "buscar_dados".to_string(),
                args: serde_json::json!({"tabela": "vendas", "operacao": "count"}),
            });
        }
    }

    if q.contains("calcular")
        || q.contains("quanto é")
        || q.contains("quanto e")
        || q.contains('+')
        || q.contains('*')
    {
        // Extrai expressão simples (ex: "2 + 3")
        let expr = extract_expressao(&q).unwrap_or_else(|| "2 + 2".to_string());
        calls.push(ToolCall {
            tool: "calculadora".to_string(),
            args: serde_json::json!({"expressao": expr}),
        });
    }

    if calls.is_empty() {
        // Default: busca count
        calls.push(ToolCall {
            tool: "buscar_dados".to_string(),
            args: serde_json::json!({"tabela": "vendas", "operacao": "count"}),
        });
    }

    calls
}

fn extract_expressao(q: &str) -> Option<String> {
    // Procura padrão "X + Y" etc — split_whitespace já faz trim
    for op in &['+', '*', '/', '-'] {
        if let Some(idx) = q.find(*op) {
            let left = q[..idx].split_whitespace().last()?.to_string();
            let right = q[idx + 1..].split_whitespace().next()?.to_string();
            if left.parse::<f64>().is_ok() && right.parse::<f64>().is_ok() {
                return Some(format!("{left} {op} {right}"));
            }
        }
    }
    None
}

fn executar_tool(call: &ToolCall) -> Result<String> {
    match call.tool.as_str() {
        "buscar_dados" => {
            let args: BuscaDadosArgs =
                serde_json::from_value(call.args.clone()).context("args buscar_dados")?;
            ferramenta_busca_dados(args)
        }
        "calculadora" => {
            let args: CalculadoraArgs =
                serde_json::from_value(call.args.clone()).context("args calculadora")?;
            ferramenta_calculadora(args)
        }
        _ => anyhow::bail!("ferramenta desconhecida: {}", call.tool),
    }
}

fn montar_resposta(query: &str, resultados: &[(ToolCall, String)]) -> String {
    let mut ctx = String::new();
    for (call, res) in resultados {
        ctx.push_str(&format!(
            "Ferramenta {} com args {} retornou: {}\n",
            call.tool, call.args, res
        ));
    }
    format!(
        "Pergunta: {query}\n\nContexto das ferramentas:\n{ctx}\nResposta: Com base nos dados acima, {}",
        if resultados.iter().any(|(c, _)| c.tool == "calculadora") {
            "o cálculo foi realizado e os dados da tabela foram consultados."
        } else {
            "os dados foram consultados com sucesso."
        }
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("=== Agente com Tool Calling — Módulo 07 ===\n");
    println!("Query: {}\n", args.query);

    // Mostra schemas das ferramentas (validação via schemars)
    println!("Schemas das ferramentas:");
    println!(
        "  buscar_dados: {}",
        serde_json::to_string_pretty(&schema_for!(BuscaDadosArgs))?
    );
    println!(
        "  calculadora:  {}\n",
        serde_json::to_string_pretty(&schema_for!(CalculadoraArgs))?
    );

    let calls = decidir_ferramentas(&args.query);
    println!("Agente decidiu chamar {} ferramenta(s):", calls.len());
    for c in &calls {
        println!("  - {} com {}", c.tool, c.args);
    }

    let mut resultados = Vec::new();
    for call in &calls {
        let res = executar_tool(call)?;
        println!("\n→ {}({}) = {}", call.tool, call.args, res);
        resultados.push((call.clone(), res));
    }

    let resposta = montar_resposta(&args.query, &resultados);
    println!("\n=== Resposta final ===\n{resposta}\n");
    println!("✓ Agente concluído — em produção, enviaria `resposta` para LLM via `async-openai` com function calling");

    // Demonstra inferência local (llama-cpp-rs) como alternativa — apenas comentário
    println!("\nNota: inferência local com `llama-cpp-rs` (GGUF) ou `candle` seria alternativa ao LLM remoto,");
    println!(
        "mas requer modelo GGUF (~2GB) e está marcada como 🔴 no README devido à instabilidade."
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn busca_dados_count() {
        let args = BuscaDadosArgs {
            tabela: "vendas".to_string(),
            coluna: None,
            operacao: "count".to_string(),
        };
        let res = ferramenta_busca_dados(args).expect("ok");
        assert!(res.contains("1000"));
    }

    #[test]
    fn calculadora_soma() {
        let args = CalculadoraArgs {
            expressao: "2 + 3".to_string(),
        };
        let res = ferramenta_calculadora(args).expect("ok");
        assert!(res.contains('5'));
    }

    #[test]
    fn decidir_ferramentas_detecta_count() {
        let calls = decidir_ferramentas("Quantos registros tem a tabela vendas?");
        assert!(calls.iter().any(|c| c.tool == "buscar_dados"));
    }

    #[test]
    fn schema_gerado() {
        let s = schema_for!(BuscaDadosArgs);
        assert!(s.schema.object.is_some());
    }
}
