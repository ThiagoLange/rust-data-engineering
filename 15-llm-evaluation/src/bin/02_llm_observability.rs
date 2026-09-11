//! Observabilidade de chamadas LLM: latência, tokens e custo.
//!
//! Seção correspondente no README: Exemplos → `02_llm_observability`.
//!
//! O `MedidorLlm` envolve qualquer chamada (API ou local) e acumula
//! estatísticas; `orcamento_ok` falha o pipeline se o custo estourar.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Preço em USD por 1k tokens (exemplo didático, ajuste para seu provedor).
#[derive(Debug, Clone, Copy)]
pub struct TabelaPreco {
    pub input_por_1k: f64,
    pub output_por_1k: f64,
}

impl TabelaPreco {
    pub fn exemplo() -> Self {
        Self {
            input_por_1k: 0.0015,
            output_por_1k: 0.002,
        }
    }
}

/// Uma chamada LLM registrada.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChamadaLlm {
    pub operacao: String,
    pub latencia_ms: u64,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub custo_usd: f64,
}

impl ChamadaLlm {
    pub fn nova(
        operacao: &str,
        latencia_ms: u64,
        tokens_in: u32,
        tokens_out: u32,
        preco: TabelaPreco,
    ) -> Self {
        let custo_usd = tokens_in as f64 / 1000.0 * preco.input_por_1k
            + tokens_out as f64 / 1000.0 * preco.output_por_1k;
        Self {
            operacao: operacao.to_string(),
            latencia_ms,
            tokens_in,
            tokens_out,
            custo_usd,
        }
    }
}

/// Agregador com p50/p99 de latência e totais de custo.
#[derive(Debug, Default)]
pub struct MedidorLlm {
    chamadas: Vec<ChamadaLlm>,
}

impl MedidorLlm {
    pub fn registrar(&mut self, chamada: ChamadaLlm) {
        self.chamadas.push(chamada);
    }

    pub fn total_chamadas(&self) -> usize {
        self.chamadas.len()
    }

    pub fn custo_total_usd(&self) -> f64 {
        self.chamadas.iter().map(|c| c.custo_usd).sum()
    }

    pub fn tokens_total(&self) -> u32 {
        self.chamadas
            .iter()
            .map(|c| c.tokens_in + c.tokens_out)
            .sum()
    }

    fn percentil_latencia(&self, p: f64) -> u64 {
        if self.chamadas.is_empty() {
            return 0;
        }
        let mut lat: Vec<u64> = self.chamadas.iter().map(|c| c.latencia_ms).collect();
        lat.sort_unstable();
        let idx = ((p / 100.0) * (lat.len() as f64 - 1.0)).round() as usize;
        lat[idx.min(lat.len() - 1)]
    }

    pub fn p50_ms(&self) -> u64 {
        self.percentil_latencia(50.0)
    }

    pub fn p99_ms(&self) -> u64 {
        self.percentil_latencia(99.0)
    }

    /// Falha se o custo acumulado passar do orçamento (gate de CI/CD).
    pub fn orcamento_ok(&self, limite_usd: f64) -> Result<()> {
        let total = self.custo_total_usd();
        if total > limite_usd {
            bail!("orçamento estourado: ${total:.4} > ${limite_usd:.4}");
        }
        Ok(())
    }

    pub fn resumo_json(&self) -> Result<String> {
        let v = serde_json::json!({
            "chamadas": self.total_chamadas(),
            "tokens_total": self.tokens_total(),
            "custo_total_usd": self.custo_total_usd(),
            "p50_ms": self.p50_ms(),
            "p99_ms": self.p99_ms(),
        });
        Ok(serde_json::to_string_pretty(&v)?)
    }
}

fn main() -> Result<()> {
    let preco = TabelaPreco::exemplo();
    let mut med = MedidorLlm::default();

    // Simula 5 chamadas de um pipeline RAG (embed + rerank + juiz).
    let ops = [
        ("embed_query", 120, 40, 0),
        ("embed_docs", 300, 2000, 0),
        ("rerank", 450, 1500, 50),
        ("geracao", 1800, 800, 220),
        ("juiz_faithfulness", 1500, 900, 60),
    ];
    for (op, lat, tin, tout) in ops {
        med.registrar(ChamadaLlm::nova(op, lat, tin, tout, preco));
    }

    println!("=== Observabilidade LLM ===\n{}", med.resumo_json()?);
    match med.orcamento_ok(1.00) {
        Ok(()) => println!("\norçamento OK (< $1.00)"),
        Err(e) => println!("\n{e}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custo_soma_input_output() {
        let preco = TabelaPreco {
            input_por_1k: 1.0,
            output_por_1k: 2.0,
        };
        let c = ChamadaLlm::nova("t", 10, 1000, 1000, preco);
        assert!((c.custo_usd - 3.0).abs() < 1e-9);
    }

    #[test]
    fn percentis_com_uma_chamada() {
        let mut med = MedidorLlm::default();
        med.registrar(ChamadaLlm::nova("t", 100, 10, 10, TabelaPreco::exemplo()));
        assert_eq!(med.p50_ms(), 100);
        assert_eq!(med.p99_ms(), 100);
    }

    #[test]
    fn orcamento_falha_acima_do_limite() {
        let mut med = MedidorLlm::default();
        med.registrar(ChamadaLlm::nova(
            "cara",
            10,
            1_000_000,
            0,
            TabelaPreco::exemplo(),
        ));
        assert!(med.orcamento_ok(0.01).is_err());
        assert!(med.orcamento_ok(10_000.0).is_ok());
    }
}
