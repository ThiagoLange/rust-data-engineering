# 00 — Fundamentos de Rust aplicados a dados

**Nível:** Básico
**Estado do ecossistema:** 🟢 Produção-ready (linguagem core)

## Por que Rust para dados?

Performance de C/C++ sem garbage collector, mas com segurança de memória garantida em tempo de compilação. Em engenharia de dados isso importa porque pipelines costumam ser I/O-bound *e* CPU-bound ao mesmo tempo — Rust permite paralelismo seguro sem o overhead de um GC pausando o mundo no meio de um processamento de lote grande.

## Conteúdo

1. **Ownership e borrowing na prática** — por que isso evita bugs clássicos de pipelines (double-free, data races) que em Python/Java só aparecem em produção
2. **Traits e generics** — como escrever transformações de dados reutilizáveis (`impl Transform<T>`)
3. **Error handling** — `Result<T, E>`, `?` operator, `anyhow` para prototipagem, `thiserror` para bibliotecas
4. **Concorrência**
   - Threads nativas + `std::sync` (Mutex, Arc, channels)
   - `rayon` — paralelismo de dados "quase de graça" (`.par_iter()`)
   - `tokio` — async/await para I/O concorrente (redes, arquivos)
   - Quando usar cada um: rayon para CPU-bound (transformar 10M linhas), tokio para I/O-bound (mil requisições HTTP simultâneas)

## Exemplo prático

`src/main.rs`: CLI que lê um CSV, aplica uma transformação (normalização de coluna numérica) usando `rayon` para paralelizar, e escreve o resultado — comparando tempo de execução single-thread vs paralelo.

```bash
cargo run --release -- --input dados.csv --output resultado.csv
```

## Exercício

Implemente um contador de palavras paralelo (`rayon`) que processa múltiplos arquivos de texto simultaneamente e agrega os resultados com segurança (sem `unsafe`, sem race conditions). Solução em `solucoes/00-fundamentos`.

## Leituras complementares

- [The Rust Book — Ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [Rayon docs](https://docs.rs/rayon)
- [Tokio tutorial](https://tokio.rs/tokio/tutorial)
