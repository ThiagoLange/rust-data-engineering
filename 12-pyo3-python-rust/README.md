# 12 — PyO3: Ponte Python ↔ Rust

**Nível:** Avançado
**Pré-requisito:** módulos 00–11

## Por que PyO3?

O ecossistema de dados do Python é incomparável (pandas, scikit-learn, HuggingFace). O Rust oferece performance e segurança de memória. O PyO3 permite escrever extensões Python em Rust e chamar código Rust a partir do Python — o padrão "treine em Python, sirva em Rust" dos times de dados.

## Estado do ecossistema

🟢 **Produção-ready**: `pyo3` com `maturin` para build/publish é o caminho padrão.

## Exemplos

| Binário | O que demonstra |
|---------|-----------------|
| `01_python_extension` | Função Rust exposta ao Python via PyO3: recebe JSON, agrega com Polars, devolve JSON |
| `exercicio_python_extension` | Solução: segunda `#[pyfunction]` (`resumir_transacoes`) + round-trip serde sob GIL |

```bash
# Rodar sem Python (bins puros):
cargo run --bin 01_python_extension
cargo run --bin exercicio_python_extension
cargo test

# Publicar como extensão Python (requer Python + maturin):
# maturin develop --features extension-module
# No Python:
# import rust_module; rust_module.processar_dados('{"vendas": [100,200]}')
```

## Notas de build (importante)

- `extension-module` é **opt-in** (`[features]` no `Cargo.toml`): sem ele, `cargo run`/`cargo test` linkam a libpython normalmente. Ative-o só ao gerar o `.so` — ele impede o link dos binários de teste.
- `auto-initialize` está no default: inicializa o interpretador nos testes (`Python::with_gil`).
- Se o linker reclamar de `libpython*.so` faltando (típico com conda, cuja lib fica fora do loader path), aponte para um Python com lib no path padrão:
  ```bash
  PYO3_PYTHON=/usr/bin/python3 cargo test
  ```

## O que implementar

1. **`src/bin/01_python_extension.rs`** — Função `#[pyfunction]` que recebe JSON, usa Polars para agregar, retorna JSON ao Python
2. **`solucao/exercicio_python_extension.rs`** — Exercício: segunda função (`resumir_transacoes`, agrupa por região) + teste de round-trip sob GIL

## Convenções importantes

- Usar `#[pyfunction]` e `#[pymodule]` para exportar
- `pyo3` usa GIL-bound por padrão; para paralelismo usar `pyo3::release` + `Python::with_gil`
- Não usar `.unwrap()` — converter erros para `PyErr` via `.into()`
- `Cargo.toml` precisa de features `extension-module` para publicar como .so