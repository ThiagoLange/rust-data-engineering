# 02 — Processamento de Dados

**Nível:** Intermediário  
**Estado do ecossistema:** 🟢 Produção-ready (Polars, Arrow) / 🟢 Produção-ready (DataFusion)

---

## O que você vai aprender neste módulo

Depois de estudar este módulo e rodar os exemplos, você será capaz de:

- [ ] Escolher entre Polars, DataFusion e Arrow puro para cada cenário.
- [ ] Usar `LazyFrame` do Polars para construir planos de execução otimizados.
- [ ] Aplicar expressões (`col`, `when/then/otherwise`), group-by, joins e agregações.
- [ ] Calcular métricas de janela (rolling window) e executar pipelines em streaming.
- [ ] Registrar tabelas e executar SQL com DataFusion.
- [ ] Criar e registrar UDFs em Rust no DataFusion.
- [ ] Montar `RecordBatch` e `Schema` com Apache Arrow.

---

## Pré-requisitos

Antes de começar este módulo, você deve ter:

- Concluído o [Módulo 00 — Fundamentos](../00-fundamentos) e o [Módulo 01 — I/O e Parsing](../01-io-e-parsing).
- Entendido `Result`, `?`, traits, generics, CSV e JSON.
- Rust instalado com `cargo` funcionando.

---

## Quando usar cada ferramenta

Rust tem três grandes opções para processamento de dados tabulares:

| Ferramenta | Melhor para | Por quê? |
|---|---|---|
| **Polars** | Transformação/exploração de dados com API expressiva | Parecido com pandas, lazy evaluation, otimizações automáticas |
| **DataFusion** | Interface SQL ou motores de query customizados | SQL embutido, UDFs, plano de query otimizável |
| **Arrow puro** | Infraestrutura, formatos, IPC entre serviços | Formato colunar in-memory, zero-copy entre linguagens |

> 💡 **Dica:** Polars e DataFusion são construídos sobre Apache Arrow. Entender Arrow ajuda a entender por que ambos são rápidos em operações colunares.

---

## Conteúdo

### 1. Polars

Polars é um DataFrame library escrito em Rust, projetado para performance e uso ergonômico.

#### Eager vs Lazy

- **Eager**: cada operação executa imediatamente e devolve um `DataFrame`.
- **Lazy**: as operações constroem um plano de execução (`LazyFrame`). O plano só é executado quando você chama `.collect()`.

A API lazy é preferida em pipelines reais porque permite otimizações automáticas: filtros podem ser empurrados para dentro da leitura, colunas não usadas são descartadas antes de joins, etc.

#### Expressões

Expressões (`Expr`) descrevem operações sobre colunas:

```rust
col("valor").gt(lit(100.0))           // filtro
when(cond).then(a).otherwise(b)       // if/else vetorizado
col("valor").sum().alias("total")     // agregação
```

#### Group-by, joins e agregações

```rust
lf.group_by([col("categoria")])
  .agg([col("valor").sum().alias("total")])
```

```rust
lf.join(outro, [col("cliente_id")], [col("cliente_id")], JoinType::Inner.into())
```

#### Streaming e window functions

Para datasets maiores que a RAM, use `.with_streaming(true)` antes de `.collect()`.

Rolling windows calculam métricas em uma janela deslizante sem perder a granularidade das linhas:

```rust
lf.rolling(col("timestamp"), [], opcoes)
  .agg([col("metrica").mean().alias("media_movel")])
```

Veja os exemplos [`01_polars_eager_lazy.rs`](./src/bin/01_polars_eager_lazy.rs), [`02_polars_groupby_join.rs`](./src/bin/02_polars_groupby_join.rs) e [`03_polars_streaming_window.rs`](./src/bin/03_polars_streaming_window.rs).

---

### 2. DataFusion

DataFusion é um motor de query SQL construído sobre Arrow. Ele permite:

- registrar arquivos CSV/Parquet como tabelas;
- executar SQL diretamente;
- registrar UDFs escritas em Rust e chamá-las dentro do SQL.

```rust
let ctx = SessionContext::new();
ctx.register_csv("transacoes", "dados/transacoes.csv", CsvReadOptions::new()).await?;
let df = ctx.sql("SELECT categoria, SUM(valor) FROM transacoes GROUP BY categoria").await?;
```

Veja o exemplo [`04_datafusion_sql_udf.rs`](./src/bin/04_datafusion_sql_udf.rs).

---

### 3. Apache Arrow

Arrow é o formato colunar in-memory que sustenta Polars e DataFusion.

Conceitos fundamentais:

- **Schema**: descreve os nomes e tipos das colunas.
- **Array**: um vetor tipado (`Int32Array`, `StringArray`, `Float64Array`, ...).
- **RecordBatch**: um conjunto de arrays do mesmo comprimento, associados a um schema.

A vantagem do formato colunar: todos os valores de uma coluna ficam contíguos na memória, o que permite operações vetorizadas eficientes e compartilhamento zero-copy entre processos e linguagens.

Veja o exemplo [`05_arrow_basico.rs`](./src/bin/05_arrow_basico.rs).

---

## Como estudar este módulo

1. Leia cada seção acima antes de abrir o exemplo.
2. Abra o código em `src/bin/NN_nome.rs` e leia os comentários de cabeçalho.
3. Rode o exemplo:
   ```bash
   cargo run --bin 01_polars_eager_lazy
   ```
4. Rode os testes:
   ```bash
   cargo test
   ```
5. Só depois vá para o próximo exemplo.

---

## Exemplos

| # | Binário | Conceito-chave |
|---|---------|----------------|
| 1 | [`01_polars_eager_lazy.rs`](./src/bin/01_polars_eager_lazy.rs) | Eager vs `LazyFrame`, `.explain()`, `when/then/otherwise` |
| 2 | [`02_polars_groupby_join.rs`](./src/bin/02_polars_groupby_join.rs) | Group-by, joins, agregações |
| 3 | [`03_polars_streaming_window.rs`](./src/bin/03_polars_streaming_window.rs) | Streaming, rolling window, datas |
| 4 | [`04_datafusion_sql_udf.rs`](./src/bin/04_datafusion_sql_udf.rs) | SQL embutido e UDFs |
| 5 | [`05_arrow_basico.rs`](./src/bin/05_arrow_basico.rs) | `RecordBatch`, `Schema`, arrays |
| 6 | [`polars_pipeline.rs`](./src/bin/polars_pipeline.rs) | ETL completo com Polars → Parquet |
| 7 | [`datafusion_pipeline.rs`](./src/bin/datafusion_pipeline.rs) | Mesmo ETL com DataFusion/SQL → Parquet |

```bash
cargo run --bin 01_polars_eager_lazy
cargo run --bin 02_polars_groupby_join
cargo run --bin 03_polars_streaming_window
cargo run --bin 04_datafusion_sql_udf
cargo run --bin 05_arrow_basico
cargo run --bin polars_pipeline
cargo run --bin datafusion_pipeline
```

Cada binário tem testes unitários no próprio arquivo (`#[cfg(test)] mod tests`). Rode `cargo test` para ver todos passando de uma vez.

---

## Exercício

Implemente uma agregação de janela deslizante (rolling window) sobre uma série temporal usando `LazyFrame` do Polars.

### Requisitos

- [ ] Leia `dados/series_temporais.csv` com Polars `LazyFrame`.
- [ ] Converta a coluna `timestamp` para o tipo `Date`.
- [ ] Calcule a média móvel de 7 dias da coluna `metrica`.
- [ ] Mantenha a coluna original `metrica` ao lado da média móvel.
- [ ] Mostre o plano de execução lazy com `.explain()`.
- [ ] Escreva o resultado em `dados/saida/rolling_window.parquet`.
- [ ] Imprima as primeiras 20 linhas.

### Dicas

- Use `col("timestamp").str().to_date(...)` para converter strings em datas.
- Use `lf.rolling(col("timestamp"), [], RollingGroupOptions { ... }).agg([...])`.
- O `period` da janela pode ser `Duration::parse("7d")`.
- Use `col("metrica").first().alias("metrica")` dentro da agregação para manter o valor original.

### Como rodar

```bash
cargo run --release --bin exercicio_rolling_window
```

### Solução

A solução está em [`solucao/exercicio_rolling_window.rs`](./solucao/exercicio_rolling_window.rs). Só olhe depois de tentar implementar sozinho.

---

## Checklist de aprendizado

Antes de ir para o módulo 03, confira se você consegue:

- [ ] Explicar a diferença entre eager e lazy no Polars.
- [ ] Ler um CSV e aplicar filtros, group-by e joins com Polars.
- [ ] Calcular uma métrica de rolling window preservando a granularidade.
- [ ] Registrar uma tabela e executar SQL com DataFusion.
- [ ] Criar e usar uma UDF no DataFusion.
- [ ] Montar um `RecordBatch` com Apache Arrow.
- [ ] Escolher entre Polars, DataFusion e Arrow puro para um cenário novo.

Se algum item ficou nebuloso, volte ao exemplo correspondente e mexa no código.

---

## Leituras complementares

- [Polars User Guide](https://docs.pola.rs/)
- [DataFusion docs](https://datafusion.apache.org/)
- [Arrow columnar format spec](https://arrow.apache.org/docs/format/Columnar.html)
