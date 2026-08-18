# 02 — Processamento de Dados

**Nível:** Intermediário
**Estado do ecossistema:** 🟢 Produção-ready (Polars, Arrow) / 🟢 Produção-ready (DataFusion)

## Conteúdo

1. **Polars**
   - DataFrame API (eager) vs LazyFrame (lazy evaluation com otimização de query plan)
   - Expressões (`col()`, `when/then/otherwise`, window functions)
   - Group-by, joins, agregações
   - Streaming de datasets maiores que a memória (`collect(streaming=true)`)
2. **DataFusion**
   - Motor de SQL embutido em Rust, construído sobre Arrow
   - Registrar tabelas e rodar SQL direto sobre Parquet/CSV
   - UDFs (User Defined Functions) escritas em Rust nativo
3. **Arrow-rs**
   - Formato colunar in-memory, zero-copy entre processos/linguagens
   - Por que Polars e DataFusion são construídos sobre Arrow
   - RecordBatch, Schema, tipos de array

## Quando usar cada um

- **Polars**: API ergonômica tipo pandas, ótimo para transformação/exploração
- **DataFusion**: quando você quer expor uma interface SQL, ou construir seu próprio motor de query customizado (UDFs, otimizações específicas)
- **Arrow puro**: quando você está construindo infraestrutura (formatos, IPC entre serviços) — como no AI-Lake

## Exemplo prático 1: ETL comparativo

`src/polars_pipeline.rs` e `src/datafusion_pipeline.rs`: o mesmo pipeline (ler CSV → filtrar → agregar → escrever Parquet) implementado nas duas abordagens, com benchmark de tempo de execução lado a lado.

```bash
cargo run --bin polars_pipeline
cargo run --bin datafusion_pipeline
```

## Exemplo prático 2: UDF em DataFusion

`src/datafusion_udf.rs`: registrar uma função customizada em Rust (ex: normalização de texto) e chamá-la via SQL.

## Exercício

Implemente uma agregação de janela deslizante (rolling window) sobre uma série temporal usando LazyFrame do Polars, comparando o plano de execução antes/depois de otimizações do lazy engine (`.explain()`). Solução em `solucoes/02-processamento-dados`.

## Leituras complementares

- [Polars User Guide](https://docs.pola.rs/)
- [DataFusion docs](https://datafusion.apache.org/)
- [Arrow columnar format spec](https://arrow.apache.org/docs/format/Columnar.html)
