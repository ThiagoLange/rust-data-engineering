#![deny(warnings)]
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
enum BackfillError {
    #[error("ciclo detectado no DAG")]
    CicloDetectado,
    #[error("janela {0} não encontrada")]
    JanelaNotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Janela {
    inicio: String,
    fim: String,
    tarefas: Vec<String>,
}

#[derive(Debug)]
struct BackfillPipeline {
    grafos: HashMap<String, DiGraph<String, ()>>,
    nomes: HashMap<String, HashMap<String, NodeIndex>>,
}

impl BackfillPipeline {
    fn nova_janela(&mut self, janela: Janela) {
        let mut graph = DiGraph::new();
        let mut nomes = HashMap::new();

        for tarefa in &janela.tarefas {
            let idx = graph.add_node(tarefa.clone());
            nomes.insert(tarefa.clone(), idx);
        }

        // Dependências implícitas: tarefa[i] depende de tarefa[i-1]
        for i in 1..janela.tarefas.len() {
            let de = nomes[&janela.tarefas[i - 1]];
            let para = nomes[&janela.tarefas[i]];
            graph.add_edge(de, para, ());
        }

        self.grafos.insert(janela.inicio.clone(), graph);
        self.nomes.insert(janela.inicio.clone(), nomes);
    }

    fn executar_janela(&self, inicio: &str) -> Result<Vec<String>, BackfillError> {
        let graph = self
            .grafos
            .get(inicio)
            .ok_or_else(|| BackfillError::JanelaNotFound(inicio.to_string()))?;

        let sorted = toposort(graph, None).map_err(|_| BackfillError::CicloDetectado)?;
        Ok(sorted.iter().map(|idx| graph[*idx].clone()).collect())
    }
}

fn main() -> anyhow::Result<()> {
    println!("=== Pipeline Backfill ===");

    let mut pipeline = BackfillPipeline {
        grafos: HashMap::new(),
        nomes: HashMap::new(),
    };

    let janelas = vec![
        Janela {
            inicio: "2024-01".into(),
            fim: "2024-02".into(),
            tarefas: vec!["extrair".into(), "transformar".into(), "carregar".into()],
        },
        Janela {
            inicio: "2024-02".into(),
            fim: "2024-03".into(),
            tarefas: vec!["extrair".into(), "transformar".into(), "carregar".into()],
        },
        Janela {
            inicio: "2024-03".into(),
            fim: "2024-04".into(),
            tarefas: vec!["extrair".into(), "transformar".into(), "carregar".into()],
        },
    ];

    for janela in &janelas {
        pipeline.nova_janela(janela.clone());
    }

    for janela in &janelas {
        let ordem = pipeline.executar_janela(&janela.inicio)?;
        println!("janela {} → {}", janela.inicio, ordem.join(" → "));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backfill_executa_janela() {
        let mut pipeline = BackfillPipeline {
            grafos: HashMap::new(),
            nomes: HashMap::new(),
        };
        pipeline.nova_janela(Janela {
            inicio: "2024-01".into(),
            fim: "2024-02".into(),
            tarefas: vec!["extrair".into(), "transformar".into(), "carregar".into()],
        });

        let ordem = pipeline.executar_janela("2024-01").unwrap();
        assert_eq!(ordem, vec!["extrair", "transformar", "carregar"]);
    }

    #[test]
    fn backfill_janela_inexistente() {
        let pipeline = BackfillPipeline {
            grafos: HashMap::new(),
            nomes: HashMap::new(),
        };
        assert!(pipeline.executar_janela("2024-99").is_err());
    }
}
