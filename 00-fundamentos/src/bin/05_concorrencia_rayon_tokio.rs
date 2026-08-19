// README: 00-fundamentos § "Concorrência" (rayon vs tokio)
//
// O exemplo anterior (04) mostrou as primitivas manuais (threads, Mutex,
// canais). Na prática, você quase nunca escreve isso à mão — usa uma das
// duas bibliotecas certas pro tipo de trabalho:
//
// - `rayon`: paralelismo de DADOS pra trabalho CPU-bound. `.par_iter()` no
//   lugar de `.iter()` e o rayon distribui os itens entre um thread pool
//   dimensionado pro número de cores da máquina. Ideal pra "transformar
//   10 milhões de linhas".
// - `tokio`: concorrência ASSÍNCRONA pra trabalho I/O-bound. Uma única
//   thread (ou um pool pequeno) consegue ter milhares de requisições HTTP
//   "em voo" ao mesmo tempo, porque enquanto uma está esperando resposta de
//   rede, a CPU processa outra — não tem thread nenhuma bloqueada esperando
//   à toa. Ideal pra "mil requisições HTTP simultâneas".
//
// Regra prática do README: rayon para CPU-bound, tokio para I/O-bound. Usar
// o errado não quebra, mas desperdiça a vantagem de cada abordagem (rayon
// pra I/O só ocupa threads esperando; tokio pra CPU-bound pesado numa task
// só trava as outras tasks daquele worker).

use rayon::prelude::*;
use std::time::{Duration, Instant};

/// Trabalho CPU-bound sintético: soma o quadrado de cada número. Numa
/// máquina real, isso seria por exemplo normalizar/validar cada linha de um
/// dataset gigante — trabalho que consome CPU e não espera nada externo.
fn custo_computacional(n: u64) -> u64 {
    (0..n).map(|i| i * i).sum()
}

/// Versão sequencial: um núcleo faz tudo, um item de cada vez.
fn processar_sequencial(entradas: &[u64]) -> Vec<u64> {
    entradas.iter().map(|&n| custo_computacional(n)).collect()
}

/// Versão paralela: `par_iter()` do rayon distribui os itens entre threads
/// do pool automaticamente. Mudou UMA palavra (`iter` -> `par_iter`) — essa
/// é a promessa do "paralelismo quase de graça" citada no README.
fn processar_paralelo(entradas: &[u64]) -> Vec<u64> {
    entradas
        .par_iter()
        .map(|&n| custo_computacional(n))
        .collect()
}

/// Simula uma chamada de rede (I/O-bound): não consome CPU, só espera.
/// Em código real seria `reqwest::get(...).await` ou uma query num banco.
async fn buscar_registro_remoto(id: u32) -> u32 {
    tokio::time::sleep(Duration::from_millis(50)).await;
    id * 10
}

/// Sequencial: espera uma resposta terminar pra só então pedir a próxima.
/// Com N requisições de 50ms cada, isso leva ~N * 50ms.
async fn buscar_todos_sequencial(ids: &[u32]) -> Vec<u32> {
    let mut resultados = Vec::with_capacity(ids.len());
    for &id in ids {
        resultados.push(buscar_registro_remoto(id).await);
    }
    resultados
}

/// Concorrente: dispara todas as requisições "ao mesmo tempo" via
/// `tokio::spawn` (uma task independente por requisição) e só então espera
/// cada uma terminar. Com N requisições de 50ms, isso leva ~50ms no total
/// (não N * 50ms) — é aqui que tokio ganha de goleada de I/O-bound.
///
/// Evitamos o crate `futures` (só pra ter `join_all`) pra manter as
/// dependências do módulo mínimas, como pede o CLAUDE.md do repo —
/// `tokio::spawn` sozinho já resolve.
async fn buscar_todos_concorrente(ids: &[u32]) -> Vec<u32> {
    let handles: Vec<_> = ids
        .iter()
        .map(|&id| tokio::spawn(buscar_registro_remoto(id)))
        .collect();
    let mut resultados = Vec::with_capacity(handles.len());
    for handle in handles {
        resultados.push(
            handle
                .await
                .expect("task não deveria ter entrado em pânico"),
        );
    }
    resultados
}

#[tokio::main]
async fn main() {
    // --- CPU-bound: rayon ---
    let entradas: Vec<u64> = (0..2_000).map(|i| 5_000 + (i % 50)).collect();

    let inicio = Instant::now();
    let resultado_sequencial = processar_sequencial(&entradas);
    let duracao_sequencial = inicio.elapsed();

    let inicio = Instant::now();
    let resultado_paralelo = processar_paralelo(&entradas);
    let duracao_paralela = inicio.elapsed();

    assert_eq!(resultado_sequencial, resultado_paralelo);
    println!("[CPU-bound] sequencial: {duracao_sequencial:?} | rayon: {duracao_paralela:?}");

    // --- I/O-bound: tokio ---
    let ids: Vec<u32> = (0..10).collect();

    let inicio = Instant::now();
    let _ = buscar_todos_sequencial(&ids).await;
    let duracao_sequencial_io = inicio.elapsed();

    let inicio = Instant::now();
    let _ = buscar_todos_concorrente(&ids).await;
    let duracao_concorrente_io = inicio.elapsed();

    println!("[I/O-bound] sequencial: {duracao_sequencial_io:?} | concorrente: {duracao_concorrente_io:?}");
    assert!(duracao_concorrente_io < duracao_sequencial_io);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequencial_e_paralelo_produzem_o_mesmo_resultado() {
        let entradas = vec![10, 20, 30, 40];
        assert_eq!(
            processar_sequencial(&entradas),
            processar_paralelo(&entradas)
        );
    }

    #[test]
    fn custo_computacional_soma_quadrados() {
        // 0^2 + 1^2 + 2^2 + 3^2 = 0 + 1 + 4 + 9 = 14
        assert_eq!(custo_computacional(4), 14);
    }

    #[tokio::test]
    async fn busca_sequencial_e_concorrente_retornam_mesmos_dados() {
        let ids = vec![1, 2, 3];
        let seq = buscar_todos_sequencial(&ids).await;
        let conc = buscar_todos_concorrente(&ids).await;
        assert_eq!(seq, conc);
        assert_eq!(seq, vec![10, 20, 30]);
    }
}
