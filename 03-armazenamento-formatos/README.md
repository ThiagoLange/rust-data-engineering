# 03 — Armazenamento e Formatos

**Nível:** Intermediário
**Estado do ecossistema:** 🟢 Parquet/Arrow · 🟢 Avro · 🟢 Arrow IPC · 🟢 delta-rs · 🟡 iceberg-rust (0.10) · 🟢 object_store · 🟡 AI Lake (Parquet + HNSW footer)

## Conteúdo

1. **Parquet** — formato colunar comprimido, o padrão para armazenamento analítico. Crate `parquet`, integração com Arrow
2. **Avro** — formato orientado a linha com schema evolution, comum em pipelines de streaming (Kafka)
3. **Arrow IPC** — formato para transferência rápida entre processos (memória compartilhada, mmap, streaming)
4. **delta-rs** — implementação Rust nativa do protocolo Delta Lake: transações ACID, time travel, merge/upsert
5. **iceberg-rust** — implementação da spec Apache Iceberg v2: particionamento oculto, evolução de schema sem reescrever dados
6. **object_store crate** — abstração unificada para S3, GCS, Azure Blob — a mesma API não importa o backend
7. **AI Lake** — formato lakehouse vetorial: Parquet colunar + footer HNSW + metadados Iceberg/Delta (base do Projeto 11)

## Delta Lake vs Iceberg: o que escolher

Ambos resolvem o mesmo problema (transações ACID sobre object storage), com trade-offs diferentes de ecossistema, compatibilidade de engines e maturidade das bibliotecas Rust — cobrimos os dois lado a lado via `04_delta_lake_basico` e `08_iceberg_basico` para você decidir com base no seu caso. AI Lake estende ambos com índice vetorial nativo no footer Parquet.

## Exemplos práticos

| Binário | Arquivo | O que demonstra |
|---------|---------|----------------|
| `01_parquet_escrita_leitura` | `src/bin/01_parquet_escrita_leitura.rs` | Ler CSV → `RecordBatch` Arrow → Parquet (`ArrowWriter`) e leitura de volta (`ParquetRecordBatchReaderBuilder`) |
| `02_parquet_schema_compressao` | `src/bin/02_parquet_schema_compressao.rs` | Schema explícito e comparação de compressão (Uncompressed / Snappy / Zstd) via `WriterProperties` |
| `03_parquet_com_polars` | `src/bin/03_parquet_com_polars.rs` | Conversão CSV→Parquet com Polars e leitura filtrada com predicate pushdown (`LazyFrame::scan_parquet`) |
| `04_delta_lake_basico` | `src/bin/04_delta_lake_basico.rs` | Criar tabela Delta, `Append`/`Overwrite`, `scan_table` e contagem de linhas — base para ACID/time travel |
| `05_object_store_local` | `src/bin/05_object_store_local.rs` | `object_store::local::LocalFileSystem` como bucket local (`put`/`list`/`get`) |
| `06_avro_escrita_leitura` | `src/bin/06_avro_escrita_leitura.rs` | Schema Avro (`apache-avro`), escrita com codecs Null/Deflate, leitura e datum single-record (padrão Kafka) |
| `07_arrow_ipc_escrita_leitura` | `src/bin/07_arrow_ipc_escrita_leitura.rs` | Arrow IPC File vs Stream (`FileWriter`/`StreamWriter`, `FileReader`/`StreamReader`), zero-copy/mmap |
| `08_iceberg_basico` | `src/bin/08_iceberg_basico.rs` | `MemoryCatalog` Iceberg, namespace, tabela particionada (hidden partitioning), evolução de schema |
| `09_ailake_demo` | `src/bin/09_ailake_demo.rs` | Parquet + `key_value_metadata` simulando footer HNSW — padrão AI Lake (ver Módulo 11) |
| `parquet_etl` | `src/bin/parquet_etl.rs` | Pipeline ETL: CSV → Parquet particionado manualmente por `regiao=XXX` e leitura recursiva |

```bash
cargo run --bin 01_parquet_escrita_leitura
cargo run --bin 02_parquet_schema_compressao
cargo run --bin 03_parquet_com_polars
cargo run --bin 04_delta_lake_basico
cargo run --bin 05_object_store_local
cargo run --bin 06_avro_escrita_leitura
cargo run --bin 07_arrow_ipc_escrita_leitura
cargo run --bin 08_iceberg_basico
cargo run --bin 09_ailake_demo
cargo run --bin parquet_etl
```

## Exercício

Implemente um pequeno pipeline que ingere dados incrementalmente numa tabela Delta particionada por data, com deduplicação via merge, e valide que o time travel retorna o estado correto antes/depois do merge. Solução em `solucao/exercicio_delta_merge_timetravel.rs` (bin `exercicio_delta_merge_timetravel`).

```bash
cargo run --bin exercicio_delta_merge_timetravel
```

## Leituras complementares

- [delta-rs](https://github.com/delta-io/delta-rs)
- [iceberg-rust](https://github.com/apache/iceberg-rust)
- [object_store crate](https://docs.rs/object_store)
- [apache-avro](https://docs.rs/apache-avro)
- [arrow-ipc](https://docs.rs/arrow-ipc)
- [AI Lake — Mini Lakehouse vetorial (Módulo 11)](../11-projetos-integradores)
