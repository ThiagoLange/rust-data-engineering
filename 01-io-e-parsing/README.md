# 01 — I/O e Parsing

**Nível:** Básico
**Estado do ecossistema:** 🟢 Produção-ready

## Conteúdo

1. **serde** — o padrão de fato para serialização em Rust. `Serialize`/`Deserialize` derive macros
2. **serde_json** — parsing de JSON estruturado e semi-estruturado (com `serde_json::Value` para schemas dinâmicos)
3. **csv crate** — leitura/escrita de CSV com inferência de tipos e streaming (não carrega tudo em memória)
4. **Polars para leitura tabular** — introdução rápida ao `read_csv`/`read_parquet` (aprofundamento no Módulo 2)
5. **Parsing binário com nom/winnow** — parser combinators para formatos customizados (útil para logs proprietários, protocolos binários)

## Exemplo prático 1: Parser de logs estruturados

`src/log_parser.rs`: parser que lê logs em formato semi-estruturado (ex: `[timestamp] LEVEL mensagem key=value key2=value2`), valida contra um schema com `serde`, e emite erros claros para linhas malformadas em vez de falhar silenciosamente.

## Exemplo prático 2: Parser binário com winnow

`src/binary_parser.rs`: parser de um formato binário simples (header + registros de tamanho fixo) demonstrando como `winnow` evita parsing manual de bytes propenso a erro.

```bash
cargo run --bin log_parser -- logs/app.log
cargo run --bin binary_parser -- dados.bin
```

## Exercício

Escreva um parser que valida um arquivo CSV contra um schema declarado (tipos de coluna, campos obrigatórios) e produz um relatório de linhas inválidas com número da linha e motivo do erro. Solução em `solucoes/01-io-e-parsing`.

## Leituras complementares

- [Serde docs](https://serde.rs/)
- [csv crate](https://docs.rs/csv)
- [winnow](https://docs.rs/winnow)
