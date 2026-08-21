# 01 — I/O e Parsing

**Nível:** Básico
**Estado do ecossistema:** 🟢 Produção-ready

## Conteúdo

### 1. serde

`serde` (SERialize/DEserialize) é o framework padrão de fato para converter dados entre a memória do seu programa e um formato externo. Ele mesmo não sabe ler JSON, YAML ou nada disso — define só os traits `Serialize` e `Deserialize`, que descrevem "como visitar os campos deste tipo". Quem entende o formato concreto são outros crates (`serde_json`, `serde_yaml`, `bincode`...) construídos sobre esses traits.

Na prática, você quase nunca implementa `Serialize`/`Deserialize` manualmente: usa as derive macros `#[derive(Serialize, Deserialize)]` no struct, e o compilador gera o código de conversão pra você. Dois atributos aparecem bastante em dados reais: `#[serde(rename = "...")]`, pra quando o nome do campo na origem não é um identificador Rust válido ou não segue `snake_case` (JSON com `camelCase`, ou nomes como `"e-mail"`); e `#[serde(default)]`, pra campos que podem estar ausentes — em vez de falhar o parsing, o campo assume o valor padrão do tipo. Isso importa em engenharia de dados porque schemas de origem evoluem: um campo novo não deveria quebrar o parsing de registros antigos que ainda não o têm.

### 2. serde_json com `Value` para schemas dinâmicos

Quando o schema é conhecido em tempo de compilação, um struct tipado (item 1) é a melhor escolha. Mas boa parte dos dados que chegam numa pipeline de ingestão — eventos de analytics, webhooks de terceiros, logs heterogêneos — não têm formato fixo: cada registro pode ter campos diferentes. Pra esse caso, `serde_json::Value` representa *qualquer* JSON válido como uma enum (`Object`, `Array`, `String`, `Number`, `Bool`, `Null`), navegável em runtime com `.get("campo")` (que devolve `Option`, nunca entra em pânico por um campo ausente).

`Value` troca segurança de tipos em tempo de compilação por flexibilidade em tempo de execução. A regra prática: use `Value` na borda do sistema, onde o schema realmente varia, e converta pra um struct tipado assim que possível — o resto do pipeline se beneficia da checagem do compilador em vez de espalhar `.get()`/`.as_str()` por todo o código.

### 3. csv crate

O crate `csv` lê e escreve CSV em modo *streaming*: em vez de carregar o arquivo inteiro em memória, ele expõe um iterador que decodifica uma linha por vez, sob demanda. Pra um arquivo de alguns megabytes isso não faz diferença perceptível, mas pra um de 50 GB é a diferença entre um uso de memória constante (poucos MB, não importa o tamanho do arquivo) e estourar a RAM da máquina antes mesmo de começar a processar.

Combinado com `serde`, `.deserialize::<T>()` decodifica cada linha direto pra um struct tipado, sem parsing manual de posição de coluna. E como o iterador produz um `Result` por linha, uma linha malformada no meio do arquivo vira um `Err` isolado — dá pra decidir, registro a registro, se aquilo é motivo de abortar o processamento inteiro ou só de reportar e seguir em frente.

### 4. Polars para leitura tabular

`csv` (item anterior) e `serde_json` pensam em *linha*: cada `.next()` do iterador processa um registro por vez. Polars pensa em *coluna*: um `DataFrame` guarda cada coluna como um bloco contíguo de memória do mesmo tipo (todos os `preco` juntos, todos os `nome` juntos), o que permite operações vetorizadas — somar uma coluna inteira, filtrar, agrupar — muito mais rápidas do que iterar linha a linha em código Rust puro.

Esse módulo só introduz `read_csv`/filtro/seleção via `LazyFrame`; o Módulo 2 aprofunda em planos de execução, otimização de queries lazy e formatos colunares como Parquet. Vale já notar o padrão "lazy": construir um `LazyFrame` só *descreve* as operações desejadas — nada é lido ou processado até chamar `.collect()`, momento em que o Polars pode otimizar o plano inteiro (por exemplo, aplicar um filtro durante a própria leitura, em vez de ler tudo e filtrar depois).

### 5. Parsing binário com winnow

`serde`, `csv` e Polars resolvem formatos *padronizados*. Mas dados de engenharia frequentemente vêm em formatos proprietários sem biblioteca pronta: logs customizados, protocolos binários internos, arquivos de configuração com sintaxe própria. Escrever esse parsing "na unha" — `split`, `find`, índices manuais, `u32::from_le_bytes` em fatias contadas à mão — é frágil: um erro de contagem de bytes lê o campo seguinte deslocado, silenciosamente.

*Parser combinators* (o padrão que `winnow` implementa) resolvem isso combinando parsers pequenos e testáveis — um parser de "uma palavra", um de "um número little-endian", um de "até o próximo separador" — em parsers maiores. Cada peça pequena é fácil de entender e testar isoladamente, e o combinator de mais alto nível herda essa confiabilidade. O mesmo conjunto de combinators funciona tanto sobre texto (`&str`) quanto sobre bytes (`&[u8]`), o que este módulo demonstra com dois exemplos: um parseando pares `chave=valor` em texto, outro parseando um formato binário com cabeçalho e registros de tamanho fixo.

## Exemplos

Cada ponto do "Conteúdo" acima virou um binário próprio em `src/bin/`, com código comentado pensando em quem está começando em Rust, na mesma linha do Módulo 00. Os dois exemplos práticos pedidos no enunciado original do módulo (parser de logs e parser binário) fecham a lista:

| # | Binário | O que mostra na prática |
|---|---------|--------------------------|
| 1 | [`01_serde_basico.rs`](./src/bin/01_serde_basico.rs) | `#[derive(Serialize, Deserialize)]`, `#[serde(rename)]`, `#[serde(default)]`, roundtrip serialização/deserialização |
| 2 | [`02_serde_json_dinamico.rs`](./src/bin/02_serde_json_dinamico.rs) | `serde_json::Value` navegando eventos de analytics com schemas diferentes entre registros |
| 3 | [`03_csv_streaming.rs`](./src/bin/03_csv_streaming.rs) | Leitura/escrita de CSV linha a linha via streaming, separando linhas válidas de inválidas com número da linha |
| 4 | [`04_polars_leitura_tabular.rs`](./src/bin/04_polars_leitura_tabular.rs) | `LazyFrame`, `read_csv`, filtro e seleção de colunas com Polars |
| 5 | [`05_parsing_binario_winnow.rs`](./src/bin/05_parsing_binario_winnow.rs) | Combinators básicos do winnow (`separated_pair`, `separated`, `take_till`) parseando pares `chave=valor` em texto |
| 6 | [`log_parser.rs`](./src/bin/log_parser.rs) | **Exemplo prático 1**: parser de logs semi-estruturados (`[timestamp] NIVEL mensagem key=value...`) combinando winnow (estrutura fixa) com serde (validação tipada dos campos) |
| 7 | [`binary_parser.rs`](./src/bin/binary_parser.rs) | **Exemplo prático 2**: parser de formato binário customizado (cabeçalho + registros de tamanho fixo) com os parsers binários do winnow (`le_u32`, `le_f32`, ...) |

```bash
cargo run --bin 01_serde_basico
cargo run --bin 02_serde_json_dinamico
cargo run --bin 03_csv_streaming
cargo run --bin 04_polars_leitura_tabular
cargo run --bin 05_parsing_binario_winnow
cargo run --bin log_parser
cargo run --bin binary_parser
```

Cada binário tem testes unitários no próprio arquivo (`#[cfg(test)] mod tests`), incluindo casos de entrada malformada — não só o caminho feliz. Rode `cargo test` pra ver todos passando de uma vez.

## Exercício

Escreva um parser que valida um arquivo CSV contra um schema declarado (tipos de coluna, campos obrigatórios) e produz um relatório de linhas inválidas com número da linha e motivo do erro. Solução em [`solucao/exercicio_validador_csv.rs`](./solucao/exercicio_validador_csv.rs).

## Leituras complementares

- [Serde docs](https://serde.rs/)
- [csv crate](https://docs.rs/csv)
- [winnow](https://docs.rs/winnow)
