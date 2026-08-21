# 00 — Fundamentos de Rust aplicados a dados

**Nível:** Básico
**Estado do ecossistema:** 🟢 Produção-ready (linguagem core)

## Por que Rust para dados?

Performance de C/C++ sem garbage collector, mas com segurança de memória garantida em tempo de compilação. Em engenharia de dados isso importa porque pipelines costumam ser I/O-bound *e* CPU-bound ao mesmo tempo — Rust permite paralelismo seguro sem o overhead de um GC pausando o mundo no meio de um processamento de lote grande.

## Conteúdo

### 1. Ownership e borrowing na prática

Toda outra linguagem que você já usou resolve "quem é dono desta memória" de um destes jeitos: um garbage collector que fica rastreando referências em background (Python, Java, JS), ou você mesmo gerenciando `malloc`/`free` na mão (C). Rust escolhe um terceiro caminho: **o compilador rastreia posse em tempo de compilação**, sem custo nenhum em runtime.

As regras são simples de enunciar, mas mudam como você pensa o código:

- Todo valor tem exatamente **um dono** (uma variável).
- Quando você passa esse valor pra outra função/variável sem usar `&`, a posse é **transferida** (move) — o dono anterior não pode mais usá-lo.
- Você pode **emprestar** (borrow) o valor com `&valor` (referência imutável, quantas quiser ao mesmo tempo) ou `&mut valor` (referência mutável, só uma por vez, e nunca junto com uma imutável).

Por que isso importa em dados: um pipeline típico passa um lote de registros por várias etapas (`ler → validar → transformar → gravar`). Em linguagens com GC, é fácil duas etapas acabarem segurando referência pro "mesmo" lote e uma mutar por baixo da outra — isso é uma data race, e em geral só aparece em produção, sob carga, de forma intermitente. Em Rust, esse tipo de bug **não compila**. O compilador rejeita o programa antes mesmo dele rodar.

O exemplo `01_ownership_borrowing.rs` mostra as três situações lado a lado: uma função que toma posse (`Vec<T>` por valor), uma que só empresta pra ler (`&[T]`), e uma que empresta pra mutar in-place (`&mut [T]`) — com um caso comentado de código que *não compilaria* se descomentado, pra você ver o erro que o compilador dá.

### 2. Traits e generics

**Trait** é um contrato: "todo tipo que implementar este trait tem estes métodos". Se você já usou interface (Java/TypeScript/Go) ou protocol (Python/Swift), é a mesma ideia. **Generics** é escrever uma função/struct que funciona pra qualquer tipo que satisfaça um contrato, sem repetir a lógica pra cada tipo concreto.

Isso importa em engenharia de dados porque pipelines de transformação se repetem: normalizar uma coluna numérica, formatar uma coluna de texto, validar um campo de data — é sempre "pega um valor, devolve outro valor transformado, encadeia várias dessas". Em vez de escrever um loop `for` diferente pra cada tipo de transformação, você define um trait (`Transform<T>`) e escreve o loop **uma vez**, genérico sobre `T`.

Dois detalhes que aparecem no exemplo:

- **Generics estáticos** (`fn aplicar<T, Tr: Transform<T>>(...)`): o compilador sabe exatamente qual tipo concreto é usado em cada chamada e gera código especializado pra cada um (monomorphization) — zero custo em runtime comparado a copiar e colar o loop.
- **Trait objects** (`Box<dyn Transform<f64>>`): quando a escolha de qual implementação usar só é conhecida em runtime (ex: um pipeline configurável por arquivo de config), generics estáticos não servem — você paga uma pequena indireção (vtable) em troca de flexibilidade.

### 3. Error handling

Rust não tem exceptions. Toda função que pode falhar retorna `Result<T, E>`: `Ok(valor)` ou `Err(erro)`. Não existe try/catch escondido nem exceção que escapa silenciosamente — o compilador **obriga** você a lidar com o `Result` de alguma forma antes de extrair o valor de dentro dele.

Duas ferramentas, dois papéis, e os dois aparecem no exemplo:

- **`thiserror`**, pra código reutilizável (bibliotecas, parsers, módulos internos): você define um `enum` com uma variante por tipo de erro possível. Quem chama a função pode dar `match` no `Err` e reagir diferente por variante (ex: "linha vazia eu ignoro, valor corrompido eu aborto o lote").
- **`anyhow`**, pra binários/prototipagem: um `Result<T, anyhow::Error>` genérico, com `.context("mensagem")` pra ir encadeando explicações legíveis conforme o erro sobe a pilha de chamadas. Você usa quando só quer propagar o erro pra cima e imprimir algo útil pro usuário do CLI, sem modelar cada variante.

O operador `?` no fim de uma expressão (`funcao_que_pode_falhar()?`) é o que faz a propagação: se o resultado for `Err`, a função atual já retorna esse erro pra quem a chamou; se for `Ok`, o `?` "desembrulha" o valor e o código continua. É o equivalente Rust de um `try/catch` automático, mas visível no tipo de retorno da função — dá pra saber só lendo a assinatura se ela pode falhar.

### 4. Concorrência

Rust dá duas ferramentas de alto nível pra concorrência, cada uma pro tipo certo de trabalho, e as duas são construídas sobre as mesmas primitivas de baixo nível (`std::thread`, `Arc`, `Mutex`, `mpsc::channel`) que você também vai ver neste módulo:

- **Threads nativas + `std::sync`**: `Arc<T>` ("Atomically Reference Counted") deixa várias threads serem donas do mesmo dado ao mesmo tempo; `Mutex<T>` garante que só uma thread por vez acessa o valor lá dentro. `mpsc::channel` é uma alternativa que evita Mutex por completo: threads se comunicam mandando mensagens por um canal, em vez de compartilhar memória diretamente. É a camada que `rayon` e `tokio` escondem de você — vale entender uma vez pra saber o que está acontecendo por baixo.
- **`rayon`** — paralelismo de **dados**, pra trabalho **CPU-bound**. Trocar `.iter()` por `.par_iter()` já distribui o trabalho entre um thread pool dimensionado pro número de cores da máquina. Ideal pra "transformar 10 milhões de linhas".
- **`tokio`** — concorrência **assíncrona** (`async`/`.await`), pra trabalho **I/O-bound**. Uma thread só consegue ter milhares de requisições "em voo" ao mesmo tempo, porque enquanto uma espera resposta de rede/disco, a CPU processa outra — nenhuma thread fica bloqueada esperando à toa. Ideal pra "mil requisições HTTP simultâneas".

Regra prática pra decidir qual usar: **rayon para CPU-bound, tokio para I/O-bound**. Usar o errado não quebra o código, mas desperdiça a vantagem de cada abordagem — rayon numa tarefa I/O-bound só ocupa threads inteiras esperando rede à toa; tokio com uma tarefa CPU-bound pesada dentro de uma única task trava as outras tasks daquele worker, já que não há preempção automática entre `.await` points.

## Exemplos

Cada ponto do "Conteúdo" acima virou um binário próprio em `src/bin/`, com código comentado linha a linha pensando em quem está começando em Rust — os comentários explicam o *porquê* de cada decisão, não só o *o quê*. Rode em ordem, cada um é independente:

| # | Binário | O que mostra na prática |
|---|---------|--------------------------|
| 1 | [`01_ownership_borrowing.rs`](./src/bin/01_ownership_borrowing.rs) | Move vs `&` vs `&mut` num pipeline de registros de venda; um caso comentado de código que não compilaria |
| 2 | [`02_traits_generics.rs`](./src/bin/02_traits_generics.rs) | `trait Transform<T>` implementado pra `f64` e `String`; função genérica `aplicar_em_lote`; trait object `Box<dyn Transform<f64>>` |
| 3 | [`03_error_handling.rs`](./src/bin/03_error_handling.rs) | Erro tipado com `thiserror` (`ParseRegistroError`), propagação com `?`, `anyhow::Context` no `main`, tratamento explícito com `match` |
| 4 | [`04_concorrencia_threads.rs`](./src/bin/04_concorrencia_threads.rs) | Soma paralela com `Arc<Mutex<_>>` e com `mpsc::channel`, comparando os dois padrões |
| 5 | [`05_concorrencia_rayon_tokio.rs`](./src/bin/05_concorrencia_rayon_tokio.rs) | `rayon::par_iter` vs sequencial num trabalho CPU-bound; `tokio::spawn` concorrente vs sequencial num trabalho I/O-bound simulado, com tempos medidos |
| 6 | [`06_normalizador_csv.rs`](./src/bin/06_normalizador_csv.rs) | **Exemplo prático do módulo**: CLI completo que lê `dados/vendas.csv`, normaliza a coluna `valor` (min-max), compara tempo sequencial vs rayon, e escreve o resultado |

```bash
cargo run --bin 01_ownership_borrowing
cargo run --bin 02_traits_generics
cargo run --bin 03_error_handling
cargo run --bin 04_concorrencia_threads
cargo run --bin 05_concorrencia_rayon_tokio
cargo run --release --bin 06_normalizador_csv -- --input dados/vendas.csv --output resultado.csv
```

Cada binário tem 2-4 testes unitários no próprio arquivo (`#[cfg(test)] mod tests`), cobrindo o caminho feliz e casos de borda (ex: divisão por zero na normalização, valor não-numérico no parse). Rode `cargo test` pra ver todos passando de uma vez.

## Exercício

Implemente um contador de palavras paralelo (`rayon`) que processa múltiplos arquivos de texto simultaneamente e agrega os resultados com segurança (sem `unsafe`, sem race conditions). Solução em [`solucao/exercicio_contador_palavras.rs`](./solucao/exercicio_contador_palavras.rs).

## Leituras complementares

- [The Rust Book — Ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [Rayon docs](https://docs.rs/rayon)
- [Tokio tutorial](https://tokio.rs/tokio/tutorial)
