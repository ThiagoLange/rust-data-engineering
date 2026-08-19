// README: 00-fundamentos § "Concorrência" (threads nativas + std::sync)
//
// Antes de usar rayon ou tokio (próximo exemplo), vale entender as
// primitivas que eles usam por baixo: `std::thread`, `Arc` e `Mutex`.
//
// - `Arc<T>` ("Atomically Reference Counted"): permite que VÁRIAS threads
//   sejam donas do mesmo dado ao mesmo tempo, contando quantas referências
//   existem e liberando a memória só quando a última é derrubada. É a
//   versão thread-safe do `Rc<T>`.
// - `Mutex<T>`: garante que só uma thread por vez pode acessar o valor lá
//   dentro (`.lock()` bloqueia até conseguir acesso exclusivo). Sem isso,
//   duas threads incrementando o mesmo contador ao mesmo tempo perderiam
//   incrementos (data race clássica).
//
// O compilador Rust RECUSA compilar código que compartilha dados mutáveis
// entre threads sem essa proteção — não é convenção, é tipo (`Send`/`Sync`)
// checado em compile time. Isso é o que o README quer dizer com "concorrência
// segura sem overhead de GC pausando o mundo".

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

/// Soma valores em paralelo usando N threads nativas + um contador
/// protegido por `Mutex`, compartilhado via `Arc`.
///
/// Isso é deliberadamente "manual" — na prática, pra esse caso de uso,
/// `rayon` (próximo exemplo) faz isso melhor e com menos código. O objetivo
/// aqui é entender o mecanismo que `rayon` esconde de você.
fn somar_com_threads(valores: &[f64], num_threads: usize) -> f64 {
    let total = Arc::new(Mutex::new(0.0_f64));
    let pedacos: Vec<&[f64]> = valores
        .chunks(valores.len().div_ceil(num_threads).max(1))
        .collect();

    thread::scope(|escopo| {
        for pedaco in pedacos {
            let total = Arc::clone(&total);
            escopo.spawn(move || {
                let soma_parcial: f64 = pedaco.iter().sum();
                // `.lock()` bloqueia até nenhuma outra thread estar segurando
                // o Mutex. Enquanto seguramos o lock, temos acesso exclusivo
                // ao `f64` lá dentro — sem isso, `+=` concorrente de duas
                // threads poderia perder um dos incrementos.
                let mut total_guard = total.lock().expect("mutex não deveria estar envenenado");
                *total_guard += soma_parcial;
            });
        }
    });
    // `thread::scope` só retorna depois que TODAS as threads internas
    // terminaram — não precisamos de `.join()` manual nem corremos risco de
    // esquecer de esperar uma thread (um bug comum com `thread::spawn` cru).

    let resultado = *total.lock().expect("mutex não deveria estar envenenado");
    resultado
}

/// Padrão produtor/consumidor com `mpsc::channel` ("multi-producer,
/// single-consumer"): threads produtoras mandam mensagens por um canal,
/// uma thread consumidora recebe e agrega. Nenhum dado é compartilhado
/// diretamente — a comunicação é por mensagem, o que evita a necessidade de
/// Mutex neste padrão.
fn somar_com_canal(valores: Vec<f64>, num_produtores: usize) -> f64 {
    let (remetente, receptor) = mpsc::channel();
    let tamanho_pedaco = valores.len().div_ceil(num_produtores).max(1);

    thread::scope(|escopo| {
        for pedaco in valores.chunks(tamanho_pedaco) {
            let remetente = remetente.clone();
            escopo.spawn(move || {
                let soma_parcial: f64 = pedaco.iter().sum();
                // Enviar não bloqueia esperando um "consumidor pronto" — só
                // falha se o receptor já tiver sido derrubado.
                remetente
                    .send(soma_parcial)
                    .expect("receptor não deveria ter sido derrubado");
            });
        }
        // Precisamos derrubar nosso próprio remetente original; senão o
        // `for` abaixo nunca vê o canal "fechar" e fica esperando pra sempre.
        drop(remetente);
    });

    receptor.iter().sum()
}

fn main() {
    let valores: Vec<f64> = (1..=1000).map(f64::from).collect();

    let total_mutex = somar_com_threads(&valores, 4);
    println!("Soma via Arc<Mutex<_>> com 4 threads: {total_mutex}");

    let total_canal = somar_com_canal(valores.clone(), 4);
    println!("Soma via mpsc::channel com 4 threads: {total_canal}");

    assert_eq!(total_mutex, total_canal);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn somar_com_threads_bate_com_soma_sequencial() {
        let valores: Vec<f64> = (1..=100).map(f64::from).collect();
        let esperado: f64 = valores.iter().sum();
        assert_eq!(somar_com_threads(&valores, 8), esperado);
    }

    #[test]
    fn somar_com_threads_funciona_com_um_unico_thread() {
        let valores = vec![1.0, 2.0, 3.0];
        assert_eq!(somar_com_threads(&valores, 1), 6.0);
    }

    #[test]
    fn somar_com_canal_bate_com_soma_sequencial() {
        let valores: Vec<f64> = (1..=100).map(f64::from).collect();
        let esperado: f64 = valores.iter().sum();
        assert_eq!(somar_com_canal(valores, 8), esperado);
    }
}
