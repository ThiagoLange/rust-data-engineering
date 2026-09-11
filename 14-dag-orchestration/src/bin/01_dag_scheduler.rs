#![deny(warnings)]
use anyhow::Result;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
enum DagError {
    #[error("ciclo detectado no DAG")]
    CicloDetectado,
    #[error("nó {0} não encontrado")]
    NoNotFound(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    nome: String,
    comando: String,
    deps: Vec<String>,
}

#[derive(Debug)]
struct Dag {
    graph: DiGraph<Task, ()>,
    nome_para_idx: HashMap<String, NodeIndex>,
}

impl Dag {
    fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            nome_para_idx: HashMap::new(),
        }
    }

    fn adicionar_tarefa(&mut self, task: Task) {
        let idx = self.graph.add_node(task.clone());
        self.nome_para_idx.insert(task.nome.clone(), idx);
    }

    fn adicionar_dependencia(&mut self, de: &str, para: &str) -> Result<(), DagError> {
        let de_idx = self
            .nome_para_idx
            .get(de)
            .ok_or_else(|| DagError::NoNotFound(de.to_string()))?;
        let para_idx = self
            .nome_para_idx
            .get(para)
            .ok_or_else(|| DagError::NoNotFound(para.to_string()))?;
        self.graph.add_edge(*de_idx, *para_idx, ());
        Ok(())
    }

    fn ordem_execucao(&self) -> Result<Vec<String>, DagError> {
        let sorted = toposort(&self.graph, None).map_err(|_| DagError::CicloDetectado)?;
        Ok(sorted
            .iter()
            .map(|idx| self.graph[*idx].nome.clone())
            .collect())
    }
}

fn main() -> anyhow::Result<()> {
    println!("=== DAG Scheduler ===");

    let mut dag = Dag::new();

    dag.adicionar_tarefa(Task {
        nome: "extrair".into(),
        comando: "SELECT * FROM raw".into(),
        deps: vec![],
    });
    dag.adicionar_tarefa(Task {
        nome: "transformar".into(),
        comando: "CREATE TABLE clean AS SELECT ...".into(),
        deps: vec!["extrair".into()],
    });
    dag.adicionar_tarefa(Task {
        nome: "carregar".into(),
        comando: "INSERT INTO warehouse".into(),
        deps: vec!["transformar".into()],
    });
    dag.adicionar_tarefa(Task {
        nome: "dashboard".into(),
        comando: "REFRESH MATERIALIZED VIEW".into(),
        deps: vec!["carregar".into()],
    });

    // Define dependências explícitas via arestas do DAG
    dag.adicionar_dependencia("extrair", "transformar").unwrap();
    dag.adicionar_dependencia("transformar", "carregar")
        .unwrap();
    dag.adicionar_dependencia("carregar", "dashboard").unwrap();

    let ordem = dag.ordem_execucao()?;
    println!("Ordem de execução:");
    for (i, nome) in ordem.iter().enumerate() {
        println!("  {}. {}", i + 1, nome);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dag_linear() {
        let mut dag = Dag::new();
        dag.adicionar_tarefa(Task {
            nome: "a".into(),
            comando: "a".into(),
            deps: vec![],
        });
        dag.adicionar_tarefa(Task {
            nome: "b".into(),
            comando: "b".into(),
            deps: vec![],
        });
        dag.adicionar_tarefa(Task {
            nome: "c".into(),
            comando: "c".into(),
            deps: vec![],
        });

        // Define dependências explícitas
        dag.adicionar_dependencia("a", "b").unwrap();
        dag.adicionar_dependencia("b", "c").unwrap();

        let ordem = dag.ordem_execucao().unwrap();
        assert_eq!(ordem, vec!["a", "b", "c"]);
    }

    #[test]
    fn dag_paralelo() {
        let mut dag = Dag::new();
        dag.adicionar_tarefa(Task {
            nome: "a".into(),
            comando: "a".into(),
            deps: vec![],
        });
        dag.adicionar_tarefa(Task {
            nome: "b".into(),
            comando: "b".into(),
            deps: vec!["a".into()],
        });
        dag.adicionar_tarefa(Task {
            nome: "c".into(),
            comando: "c".into(),
            deps: vec!["a".into()],
        });

        // b e c dependem de a
        dag.adicionar_dependencia("a", "b").unwrap();
        dag.adicionar_dependencia("a", "c").unwrap();

        let ordem = dag.ordem_execucao().unwrap();
        let pos_a = ordem.iter().position(|n| n == "a").unwrap();
        let pos_b = ordem.iter().position(|n| n == "b").unwrap();
        let pos_c = ordem.iter().position(|n| n == "c").unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_a < pos_c);
    }
}
