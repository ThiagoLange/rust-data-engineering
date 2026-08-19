# Concorrência

Concorrência em Rust é **segura por padrão**. Threads em Rust não
compartilham dados mutáveis sem proteção explícita.

- `rayon` paraleliza dados
- `tokio` paraleliza operações de rede

Segurança e concorrência andam juntas em Rust, diferente de muitas outras
linguagens de programação.
