# 03 — Armazenamento e Formatos

**Nível:** Intermediário
**Estado do ecossistema:** 🟢 Parquet/Arrow · 🟡 delta-rs · 🟡 iceberg-rust

## Conteúdo

1. **Parquet** — formato colunar comprimido, o padrão para armazenamento analítico. Crate `parquet`, integração com Arrow
2. **Avro** — formato orientado a linha com schema evolution, comum em pipelines de streaming (Kafka)
3. **Arrow IPC** — formato para transferência rápida entre processos (memória compartilhada, mmap)
4. **delta-rs** — implementação Rust nativa do protocolo Delta Lake: transações ACID, time travel, merge/upsert
5. **iceberg-rust** — implementação (em maturação) da spec Apache Iceberg v2: particionamento oculto, evolução de schema sem reescrever dados
6. **object_store crate** — abstração unificada para S3, GCS, Azure Blob — a mesma API não importa o backend

## Delta Lake vs Iceberg: o que escolher

Ambos resolvem o mesmo problema (transações ACID sobre object storage), com trade-offs diferentes de ecossistema, compatibilidade de engines e maturidade das bibliotecas Rust — cobrimos os dois lado a lado para você decidir com base no seu caso.

## Exemplo prático 1: Parquet particionado

`src/parquet_write.rs`: escrever um dataset particionado por data em Parquet com compressão configurável (Snappy, LZ4, Zstd), e ler de volta com filtro de partição (predicate pushdown).

## Exemplo prático 2: Tabela Delta/Iceberg

`src/delta_table.rs` e `src/iceberg_table.rs`: criar uma tabela, fazer append incremental, rodar um merge/upsert, e demonstrar time travel (ler uma versão anterior da tabela).

```bash
cargo run --bin delta_table
cargo run --bin iceberg_table
```

## Exercício

Implemente um pequeno pipeline que ingere dados incrementalmente numa tabela Delta particionada por data, com deduplicação via merge, e valide que o time travel retorna o estado correto antes/depois do merge. Solução em `solucoes/03-armazenamento-formatos`.

## Leituras complementares

- [delta-rs](https://github.com/delta-io/delta-rs)
- [iceberg-rust](https://github.com/apache/iceberg-rust)
- [object_store crate](https://docs.rs/object_store)
