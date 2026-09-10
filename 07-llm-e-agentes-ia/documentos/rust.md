# Rust para Engenharia de Dados

Rust é uma linguagem de sistemas focada em segurança e performance. Ownership e borrowing garantem ausência de data races sem coletor de lixo.

## Concorrência

Rayon para paralelismo de dados (CPU-bound) e Tokio para I/O assíncrono. Não misture sem justificar.

## Serialização

Serde para JSON/CSV, com derive macros. Polars para DataFrames, Arrow para memória colunar.
