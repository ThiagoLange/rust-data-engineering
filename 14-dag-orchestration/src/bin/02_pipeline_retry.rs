#![deny(warnings)]
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
enum RetryError {
    #[error("tarefa falhou após {tentativas} tentativas: {motivo}")]
    Falhou { tentativas: u32, motivo: String },
    #[error("circuit breaker aberto para {tarefa}")]
    CircuitBreaker { tarefa: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Tarefa {
    nome: String,
    duracao_ms: u64,
}

#[derive(Debug)]
struct CircuitBreaker {
    falhas: HashMap<String, u32>,
    limite: u32,
}

impl CircuitBreaker {
    fn new(limite: u32) -> Self {
        Self {
            falhas: HashMap::new(),
            limite,
        }
    }

    fn registrar_falha(&mut self, tarefa: &str) {
        *self.falhas.entry(tarefa.to_string()).or_insert(0) += 1;
    }

    fn esta_aberto(&self, tarefa: &str) -> bool {
        self.falhas
            .get(tarefa)
            .map(|c| *c >= self.limite)
            .unwrap_or(false)
    }

    fn resetar(&mut self, tarefa: &str) {
        self.falhas.remove(tarefa);
    }
}

fn executar_tarefa(tarefa: &Tarefa, retry: u32) -> Result<(), RetryError> {
    // Simula falha nas primeiras tentativas e sucesso posterior
    if retry < 2 {
        Err(RetryError::Falhou {
            tentativas: retry + 1,
            motivo: format!("{} falhou (simulado)", tarefa.nome),
        })
    } else {
        println!("  ✓ {} executada ({}ms)", tarefa.nome, tarefa.duracao_ms);
        Ok(())
    }
}

fn executar_com_retry(
    tarefa: &Tarefa,
    max_retries: u32,
    cb: &mut CircuitBreaker,
) -> Result<(), RetryError> {
    if cb.esta_aberto(&tarefa.nome) {
        return Err(RetryError::CircuitBreaker {
            tarefa: tarefa.nome.clone(),
        });
    }

    let mut tentativas = 0;
    loop {
        match executar_tarefa(tarefa, tentativas) {
            Ok(()) => {
                cb.resetar(&tarefa.nome);
                return Ok(());
            }
            Err(e) => {
                tentativas += 1;
                cb.registrar_falha(&tarefa.nome);
                if tentativas >= max_retries {
                    return Err(e);
                }
                let backoff_ms = 2u64.pow(tentativas) * 10;
                println!("  ⏳ retry {} em {}ms (backoff)", tentativas, backoff_ms);
                // Sleep simples — em produção usar tokio::time::sleep
                std::thread::sleep(std::time::Duration::from_millis(backoff_ms.min(100)));
            }
        }
    }
}

fn main() -> Result<()> {
    println!("=== Pipeline com Retry ===");

    let tarefas = vec![
        Tarefa {
            nome: "extrair_dados".into(),
            duracao_ms: 100,
        },
        Tarefa {
            nome: "transformar".into(),
            duracao_ms: 200,
        },
        Tarefa {
            nome: "carregar".into(),
            duracao_ms: 150,
        },
    ];

    let mut cb = CircuitBreaker::new(3);

    for tarefa in &tarefas {
        println!("Executando: {}", tarefa.nome);
        match executar_com_retry(tarefa, 4, &mut cb) {
            Ok(()) => println!("  ✓ sucesso"),
            Err(e) => println!("  ✗ falha: {:?}", e),
        }
    }

    println!("\nTestando circuit breaker:");
    let t = Tarefa {
        nome: "falha_constante".into(),
        duracao_ms: 50,
    };
    for i in 0..4 {
        println!("  tentativa {}", i + 1);
        match executar_com_retry(&t, 4, &mut cb) {
            Ok(()) => {}
            Err(e) => println!("  → {:?}", e),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circuit_breaker_abre() {
        let mut cb = CircuitBreaker::new(2);
        cb.registrar_falha("x");
        cb.registrar_falha("x");
        assert!(cb.esta_aberto("x"));
    }

    #[test]
    fn circuit_breaker_fechado_inicialmente() {
        let cb = CircuitBreaker::new(2);
        assert!(!cb.esta_aberto("y"));
    }

    #[test]
    fn circuit_breaker_resetar() {
        let mut cb = CircuitBreaker::new(2);
        cb.registrar_falha("x");
        cb.registrar_falha("x");
        cb.resetar("x");
        assert!(!cb.esta_aberto("x"));
    }
}
