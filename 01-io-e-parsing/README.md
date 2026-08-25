# 01 — I/O e Parsing

**Nível:** Básico  
**Estado do ecossistema:** 🟢 Produção-ready

---

## O que você vai aprender neste módulo

Depois de estudar este módulo e rodar os exemplos, você será capaz de:

- [ ] Ler e escrever JSON com `serde` usando structs tipados.
- [ ] Navegar JSON de schema dinâmico com `serde_json::Value`.
- [ ] Processar CSV em streaming com o crate `csv`.
- [ ] Ler e filtrar dados tabulares com Polars `LazyFrame`.
- [ ] Escrever parsers seguros com `winnow` para texto e bytes.
- [ ] Escolher entre schema em tempo de compilação (`serde`) e schema em runtime.

---

## Pré-requisitos

Antes de começar este módulo, você deve ter:

- Concluído o [Módulo 00 — Fundamentos](../00-fundamentos).
- Entendido `Result`, `?`, ownership, borrowing, traits e generics.
- Rust instalado com `cargo` funcionando.

---

## Por que I/O e parsing importam?

Engenharia de dados é, em grande parte, **mover e transformar dados entre formatos**. Você vai ler JSON de APIs, CSVs de sistemas legados, logs de aplicação, arquivos binários de equipamentos e, cada vez mais, Parquet e outros formatos colunares.

Rust oferece ferramentas para cada situação:

| Situação | Ferramenta típica | Por quê? |
|---|---|---|
| Schema fixo e conhecido | `serde` + struct | Segurança de tipos em tempo de compilação |
| Schema variável entre registros | `serde_json::Value` | Flexibilidade sem panics por campo ausente |
| Arquivos CSV grandes | `csv` crate | Streaming: não carrega tudo na RAM |
| Análise tabular rápida | Polars | Operações vetorizadas em colunas |
| Formato proprietário | `winnow` | Parsers pequenos, testáveis e combináveis |

---

## Conteúdo

### 1. serde

`serde` (SERialize/DEserialize) é o framework padrão para converter dados entre a memória do seu programa e um formato externo (JSON, YAML, TOML, binário...).

Ele não sabe ler nenhum formato sozinho. Ele define os traits `Serialize` e `Deserialize`, que descrevem "como visitar os campos deste tipo". Quem entende o formato concreto são crates como `serde_json`, `serde_yaml`, `bincode` etc.

Na prática, você usa as derive macros:

```rust
#[derive(Serialize, Deserialize)]
struct Produto {
    id: u32,
    nome: String,
}
```

#### Atributos úteis em dados reais

| Atributo | Para quê serve | Quando usar |
|---|---|---|
| `#[serde(rename = "...")]` | Muda o nome do campo no formato externo | JSON usa `camelCase` ou nomes inválidos em Rust como `"e-mail"` |
| `#[serde(default)]` | Usa o valor padrão se o campo estiver ausente | Schemas evoluem e registros antigos não têm campos novos |

> 💡 **Dica:** schemas de origem evoluem. Um campo novo não deveria quebrar o parsing de registros antigos. Use `#[serde(default)]` para isso.

Veja o exemplo [`01_serde_basico.rs`](./src/bin/01_serde_basico.rs).

---

### 2. serde_json com `Value` para schemas dinâmicos

Quando o schema é conhecido em tempo de compilação, um struct tipado é a melhor escolha. Mas muitos dados reais — eventos de analytics, webhooks de terceiros, logs heterogêneos — não têm formato fixo.

Para esses casos, `serde_json::Value` representa *qualquer* JSON válido como uma enum:

- `Object`
- `Array`
- `String`
- `Number`
- `Bool`
- `Null`

Você navega com `.get("campo")`, que devolve `Option` — nunca entra em pânico por campo ausente.

> ⚠️ **Atenção:** `Value` troca segurança de tipos em tempo de compilação por flexibilidade em runtime. Use `Value` na borda do sistema e converta para structs tipados assim que possível.

Veja o exemplo [`02_serde_json_dinamico.rs`](./src/bin/02_serde_json_dinamico.rs).

---

### 3. csv crate

O crate `csv` lê e escreve CSV em modo **streaming**: em vez de carregar o arquivo inteiro em memória, ele expõe um iterador que decodifica uma linha por vez, sob demanda.

Isso faz diferença real para arquivos grandes:

| Tamanho do CSV | Modo streaming | Carregar tudo |
|---|---|---|
| 1 MB | poucos KB de RAM | 1 MB na RAM |
| 50 GB | poucos MB de RAM | estoura a RAM |

Combinado com `serde`, `.deserialize::<T>()` decodifica cada linha direto para um struct tipado. Como o iterador produz um `Result` por linha, uma linha malformada vira um `Err` isolado — você decide se aborta o processamento ou só reporta e segue.

Veja o exemplo [`03_csv_streaming.rs`](./src/bin/03_csv_streaming.rs).

---

### 4. Polars para leitura tabular

`csv` e `serde_json` pensam em **linha**: cada `.next()` processa um registro por vez. Polars pensa em **coluna**: um `DataFrame` guarda cada coluna como um bloco contíguo de memória do mesmo tipo.

Isso permite operações vetorizadas — somar uma coluna inteira, filtrar, agrupar — muito mais rápidas do que iterar linha a linha em Rust puro.

#### O padrão "lazy"

```rust
let lf = LazyCsvReader::new("dados.csv").finish()?;
let df = lf
    .filter(col("preco").gt(lit(100.0)))
    .select([col("nome"), col("preco")])
    .collect()?;
```

Um `LazyFrame` só **descreve** as operações. Nada é lido ou processado até `.collect()`. O Polars pode otimizar o plano inteiro — por exemplo, aplicar um filtro durante a leitura, em vez de ler tudo e filtrar depois.

> 💡 **Dica:** este módulo é só uma introdução. O Módulo 02 aprofunda em planos de execução, otimização de queries lazy e formatos colunares.

Veja o exemplo [`04_polars_leitura_tabular.rs`](./src/bin/04_polars_leitura_tabular.rs).

---

### 5. Parsing binário com winnow

`serde`, `csv` e Polars resolvem formatos **padronizados**. Mas dados de engenharia frequentemente vêm em formatos proprietários: logs customizados, protocolos binários internos, arquivos de configuração com sintaxe própria.

Escrever parsing "na unha" — `split`, `find`, índices manuais, `u32::from_le_bytes` — é frágil. Um erro de contagem de bytes lê o campo seguinte deslocado, silenciosamente.

**Parser combinators** (o padrão que `winnow` implementa) resolvem isso combinando parsers pequenos e testáveis:

- um parser de "uma palavra";
- um parser de "um número little-endian";
- um parser de "até o próximo separador".

Cada peça é fácil de testar isoladamente, e o parser maior herda essa confiabilidade.

> 💡 **Dica:** o mesmo conjunto de combinators funciona sobre texto (`&str`) e sobre bytes (`&[u8]`).

Veja os exemplos [`05_parsing_binario_winnow.rs`](./src/bin/05_parsing_binario_winnow.rs), [`log_parser.rs`](./src/bin/log_parser.rs) e [`binary_parser.rs`](./src/bin/binary_parser.rs).

---

## Como estudar este módulo

1. Leia cada seção acima antes de abrir o exemplo.
2. Abra o código em `src/bin/NN_nome.rs` e leia os comentários de cabeçalho.
3. Rode o exemplo:
   ```bash
   cargo run --bin 01_serde_basico
   ```
4. Rode os testes:
   ```bash
   cargo test
   ```
5. Só depois vá para o próximo exemplo.

---

## Exemplos

Cada ponto do conteúdo virou um binário separado em `src/bin/`. Rode em ordem — cada um é independente, mas os conceitos se acumulam.

| # | Binário | Conceito-chave |
|---|---------|----------------|
| 1 | [`01_serde_basico.rs`](./src/bin/01_serde_basico.rs) | `#[derive(Serialize, Deserialize)]`, `#[serde(rename)]`, `#[serde(default)]` |
| 2 | [`02_serde_json_dinamico.rs`](./src/bin/02_serde_json_dinamico.rs) | `serde_json::Value` e schemas dinâmicos |
| 3 | [`03_csv_streaming.rs`](./src/bin/03_csv_streaming.rs) | Leitura/escrita de CSV em streaming |
| 4 | [`04_polars_leitura_tabular.rs`](./src/bin/04_polars_leitura_tabular.rs) | `LazyFrame`, `read_csv`, filtro e seleção |
| 5 | [`05_parsing_binario_winnow.rs`](./src/bin/05_parsing_binario_winnow.rs) | Combinators do winnow em texto |
| 6 | [`log_parser.rs`](./src/bin/log_parser.rs) | Parser de logs semi-estruturados: winnow + serde |
| 7 | [`binary_parser.rs`](./src/bin/binary_parser.rs) | Parser binário customizado com winnow |

```bash
cargo run --bin 01_serde_basico
cargo run --bin 02_serde_json_dinamico
cargo run --bin 03_csv_streaming
cargo run --bin 04_polars_leitura_tabular
cargo run --bin 05_parsing_binario_winnow
cargo run --bin log_parser
cargo run --bin binary_parser
```

Cada binário tem testes unitários no próprio arquivo (`#[cfg(test)] mod tests`), incluindo casos de entrada malformada. Rode `cargo test` para ver todos passando de uma vez.

---

## Exercício

Escreva um parser que valida um arquivo CSV contra um schema declarado e produz um relatório de linhas inválidas com número da linha e motivo do erro.

### Requisitos

- [ ] Aceite o caminho do CSV via `--input` (default: `dados/produtos.csv`).
- [ ] Aceite o schema via `--schema` no formato `coluna:tipo:obrigatorio,coluna2:tipo2:obrigatorio2` (default: schema dos produtos).
- [ ] Os tipos suportados são: `texto`, `inteiro`, `decimal`, `booleano`.
- [ ] A obrigatoriedade deve ser `true` ou `false`.
- [ ] Valide cada linha do CSV contra o schema.
- [ ] Reporte linhas inválidas com o número da linha e o motivo do erro.
- [ ] Não aborte o processamento ao encontrar uma linha inválida — continue e reporte todas.

### Formato do schema

```text
id:inteiro:true,nome:texto:true,preco:decimal:true,em_estoque:booleano:false
```

### Como rodar

```bash
# Schema padrão (dados/produtos.csv)
cargo run --release --bin exercicio_validador_csv

# Schema customizado
cargo run --release --bin exercicio_validador_csv -- \
  --input dados/produtos.csv \
  --schema "id:inteiro:true,nome:texto:true"
```

### Dicas

- Use `csv::ReaderBuilder::new().flexible(true)` para não abortar quando uma linha tiver número de campos diferente do header.
- Use um `BTreeMap<String, String>` para mapear coluna → valor bruto de cada linha.
- Valide tipo a tipo com `.parse::<i64>()`, `.parse::<f64>()`, `.parse::<bool>()`.

### Solução

A solução está em [`solucao/exercicio_validador_csv.rs`](./solucao/exercicio_validador_csv.rs). Só olhe depois de tentar implementar sozinho.

---

## Checklist de aprendizado

Antes de ir para o módulo 02, confira se você consegue:

- [ ] Explicar quando usar `serde` struct tipado vs. `serde_json::Value`.
- [ ] Usar `#[serde(rename)]` e `#[serde(default)]`.
- [ ] Explicar por que o crate `csv` é adequado para arquivos grandes.
- [ ] Separar linhas válidas de inválidas em CSV usando streaming.
- [ ] Explicar o que `LazyFrame` otimiza antes de `.collect()`.
- [ ] Escrever um parser simples com winnow combinando parsers pequenos.
- [ ] Escolher a ferramenta certa para cada formato de dado.

Se algum item ficou nebuloso, volte ao exemplo correspondente e mexa no código.

---

## Leituras complementares

- [Serde docs](https://serde.rs/)
- [csv crate](https://docs.rs/csv)
- [Polars book](https://docs.pola.rs/)
- [winnow](https://docs.rs/winnow)
