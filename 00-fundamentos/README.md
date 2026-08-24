# 00 — Fundamentos de Rust aplicados a dados

**Nível:** Básico  
**Estado do ecossistema:** 🟢 Produção-ready (linguagem core)

---

## O que você vai aprender neste módulo

Depois de estudar este módulo e rodar os exemplos, você será capaz de:

- [ ] Explicar **ownership**, **borrowing** e **lifetimes** com suas próprias palavras.
- [ ] Decidir quando passar um valor por valor (`Vec<T>`), por referência (`&[T]`) ou por referência mutável (`&mut [T]`).
- [ ] Criar e usar **traits** e **generics** para escrever código reutilizável.
- [ ] Escolher entre `thiserror` e `anyhow` para tratar erros de forma idiomática.
- [ ] Usar `Result` e o operador `?` para propagar erros sem `try/catch`.
- [ ] Escolher entre threads nativas, `rayon` e `tokio` para cada tipo de trabalho.
- [ ] Ler e escrever arquivos CSV com Rust de forma segura e performática.

---

## Pré-requisitos

Antes de começar, você precisa saber:

- O básico de programação (variáveis, funções, loops, condicionais).
- Ter o Rust instalado (`rustup`, `cargo`). Se ainda não tiver, siga [rustup.rs](https://rustup.rs).

**Não precisa saber** de Rust antecipadamente — este módulo começa do zero.

---

## Por que Rust para dados?

Rust oferece **performance de C/C++** sem garbage collector, mas com **segurança de memória garantida em tempo de compilação**.

Em engenharia de dados, isso faz diferença real:

| Cenário | Com GC (Python/Java/Go) | Com Rust |
|---|---|---|
| Processar 10 milhões de linhas | O GC pode pausar tudo no meio do pipeline | Sem pausas, memória previsível |
| Pipeline multi-etapa | Duas etapas podem mutar o mesmo lote por engano | O compilador impede isso antes de rodar |
| Servidor com milhares de conexões | Threads ou GC consomem muita memória | Async com `tokio` é leve e seguro |

> **Exemplo prático:** imagine um pipeline `ler → validar → transformar → gravar`. Em linguagens com GC, é fácil duas etapas segurarem referência pro mesmo lote e uma mutar por baixo da outra. Isso é uma *data race* que costuma aparecer só em produção, sob carga. Em Rust, esse tipo de bug **não compila**.

---

## Conteúdo

### 1. Ownership e borrowing na prática

Toda linguagem resolve "quem é dono desta memória" de um destes jeitos:

1. **Garbage collector** fica rastreando referências em background (Python, Java, JavaScript).
2. **Você gerencia na mão** com `malloc`/`free` (C, C++).
3. **Rust:** o compilador rastreia a posse em tempo de compilação — sem custo em runtime.

#### As três regras do ownership

> 📦 **Regra 1:** todo valor tem exatamente **um dono**.
>
> 📦 **Regra 2:** quando você passa o valor sem `&`, a posse é **transferida** (*move*). O dono anterior não pode mais usá-lo.
>
> 📦 **Regra 3:** você pode emprestar com `&valor` (imutável, vários ao mesmo tempo) ou `&mut valor` (mutável, só um por vez, e nunca junto com imutáveis).

#### Quando usar cada um?

| Sintaxe | Significa | Use quando... |
|---|---|---|
| `valor` | **Move**: transfere a posse | A função vai consumir o dado e você não precisa mais dele |
| `&valor` | **Borrow imutável**: empresta pra ler | Só quer ler, quer continuar usando depois |
| `&mut valor` | **Borrow mutável**: empresta pra alterar | Precisa modificar in-place sem criar cópia |

> 💡 **Dica:** prefira `&[T]` em vez de `Vec<T>` quando a função só vai ler. Assim você não força quem chama a transferir a posse do vetor.

> ⚠️ **Erro comum:** tentar usar uma variável depois de movê-la. O compilador vai reclamar com `value borrowed here after move`. Isso é uma feature, não um bug — ele está te protegendo.

O exemplo [`01_ownership_borrowing.rs`](./src/bin/01_ownership_borrowing.rs) mostra as três situações lado a lado, com um caso comentado de código que *não compilaria* se descomentado.

---

### 2. Traits e generics

**Trait** é um contrato: "todo tipo que implementar este trait tem estes métodos". Se você já usou `interface` (Java/TypeScript/Go) ou `protocol` (Python/Swift), é a mesma ideia.

**Generics** é escrever uma função ou struct que funciona para qualquer tipo que satisfaça um contrato, sem repetir código.

#### Por que isso importa em dados?

Pipelines de transformação se repetem:

- normalizar uma coluna numérica;
- formatar uma coluna de texto;
- validar um campo de data.

Em vez de escrever um `for` diferente para cada transformação, você define um trait (`Transform<T>`) e escreve o loop **uma vez**, genérico sobre `T`.

#### Dois padrões que você vai ver

1. **Generics estáticos** (`fn aplicar<T, Tr: Transform<T>>(...)`):
   - O compilador sabe o tipo concreto em cada chamada.
   - Gera código especializado para cada uso (*monomorphization*).
   - **Zero custo em runtime** comparado a copiar e colar o loop.

2. **Trait objects** (`Box<dyn Transform<f64>>`):
   - A escolha da implementação só é conhecida em runtime.
   - Útil para pipelines configuráveis por arquivo.
   - Paga uma pequena indireção (*vtable*) em troca de flexibilidade.

> 💡 **Dica:** comece com generics estáticos. Só use `Box<dyn ...>` quando realmente precisar de escolha em runtime.

Veja o exemplo [`02_traits_generics.rs`](./src/bin/02_traits_generics.rs).

---

### 3. Error handling

Rust não tem exceptions. Toda função que pode falhar retorna `Result<T, E>`:

- `Ok(valor)` quando dá certo;
- `Err(erro)` quando dá errado.

O compilador **obriga** você a lidar com o `Result` antes de usar o valor de dentro.

#### Duas ferramentas, dois papéis

| Ferramenta | Quando usar | Exemplo no módulo |
|---|---|---|
| **`thiserror`** | Código reutilizável (bibliotecas, parsers) | Definir `ParseRegistroError` com variantes específicas |
| **`anyhow`** | Binários e prototipagem | No `main`, propagar erros com mensagens de contexto |

#### O operador `?`

```rust
let valor = funcao_que_pode_falhar()?;
```

- Se o resultado for `Err`, a função atual retorna esse erro.
- Se for `Ok`, o `?` "desembrulha" o valor e continua.

> 💡 **Dica:** leia a assinatura da função. Se ela retorna `Result<T, E>`, você sabe que pode falhar. Não precisa adivinhar.

> ⚠️ **Erro comum:** usar `.unwrap()` fora de testes. Em produção, `unwrap` vira pânico. Use `?`, `match` ou `.context()`.

Veja o exemplo [`03_error_handling.rs`](./src/bin/03_error_handling.rs).

---

### 4. Concorrência

Rust oferece três níveis de concorrência. Escolha o certo para cada trabalho:

| Abordagem | Tipo de trabalho | Quando usar |
|---|---|---|
| **Threads nativas + `std::sync`** | Controle manual | Quando você precisa entender o que `rayon` e `tokio` escondem |
| **`rayon`** | Paralelismo de dados, CPU-bound | Transformar milhões de linhas, cálculos numéricos |
| **`tokio`** | Assíncrono, I/O-bound | Milhares de requisições HTTP, leitura de rede/disco |

#### Threads nativas (`std::sync`)

- `Arc<T>`: várias threads "dono" do mesmo dado ao mesmo tempo.
- `Mutex<T>`: só uma thread por vez acessa o valor.
- `mpsc::channel`: threads se comunicam por mensagens, sem compartilhar memória.

Essas primitivas são a base do `rayon` e do `tokio`. Vale entender uma vez.

#### Regra prática

> **rayon para CPU-bound, tokio para I/O-bound.**
>
> Usar o errado não quebra o código, mas desperdiça a vantagem: rayon em I/O ocupa threads esperando rede; tokio com CPU pesado numa única task trava as outras tasks.

Veja os exemplos [`04_concorrencia_threads.rs`](./src/bin/04_concorrencia_threads.rs) e [`05_concorrencia_rayon_tokio.rs`](./src/bin/05_concorrencia_rayon_tokio.rs).

---

## Como estudar este módulo

1. Leia cada seção acima **antes** de abrir o exemplo.
2. Abra o código em `src/bin/NN_nome.rs` e leia os comentários. Eles explicam o *porquê*, não só o *o quê*.
3. Rode o exemplo:
   ```bash
   cargo run --bin 01_ownership_borrowing
   ```
4. Mude algo e veja o que o compilador diz. Rust é uma ótima ferramenta de aprendizado porque os erros são específicos.
5. Rode os testes:
   ```bash
   cargo test
   ```
6. Só depois vá para o próximo exemplo.

---

## Exemplos

Cada ponto do conteúdo virou um binário separado em `src/bin/`. Rode em ordem — cada um é independente, mas os conceitos se acumulam.

| # | Binário | Conceito-chave |
|---|---------|----------------|
| 1 | [`01_ownership_borrowing.rs`](./src/bin/01_ownership_borrowing.rs) | Move vs `&` vs `&mut` |
| 2 | [`02_traits_generics.rs`](./src/bin/02_traits_generics.rs) | `trait Transform<T>`, generics e trait objects |
| 3 | [`03_error_handling.rs`](./src/bin/03_error_handling.rs) | `thiserror`, `anyhow`, `?`, `match` |
| 4 | [`04_concorrencia_threads.rs`](./src/bin/04_concorrencia_threads.rs) | `Arc<Mutex<_>>` e `mpsc::channel` |
| 5 | [`05_concorrencia_rayon_tokio.rs`](./src/bin/05_concorrencia_rayon_tokio.rs) | `rayon` para CPU-bound, `tokio` para I/O-bound |
| 6 | [`06_normalizador_csv.rs`](./src/bin/06_normalizador_csv.rs) | CLI completo: parse CSV, normalização min-max, escrita com `BufWriter` |

```bash
cargo run --bin 01_ownership_borrowing
cargo run --bin 02_traits_generics
cargo run --bin 03_error_handling
cargo run --bin 04_concorrencia_threads
cargo run --bin 05_concorrencia_rayon_tokio
cargo run --release --bin 06_normalizador_csv -- --input dados/vendas.csv --output resultado.csv
```

Cada binário tem 2–5 testes unitários no próprio arquivo (`#[cfg(test)] mod tests`), cobrindo o caminho feliz e casos de borda. Rode `cargo test` para ver todos passando de uma vez.

---

## Exercício

Implemente um contador de palavras paralelo com `rayon`.

### Requisitos

- [ ] Leia todos os arquivos `.md` e `.txt` de um diretório passado por CLI. Se nenhum diretório for passado, use `dados/textos`.
- [ ] Conte a frequência de cada palavra em cada arquivo.
- [ ] Normalize as palavras: minúsculas e sem pontuação/símbolos de markdown (`#`, `*`, `-`, etc.).
- [ ] Agregue as contagens de todos os arquivos de forma segura — sem `unsafe`, sem `Mutex`, sem race conditions.
- [ ] Imprima o **top 10 palavras mais frequentes** e o **total de palavras distintas**.

### Dicas

- Use o padrão **map paralelo → reduce**: cada arquivo produz seu próprio `HashMap`; depois combine os mapas.
- `rayon::prelude::*` traz `.par_iter()`.
- Para ordenar pelo mais frequente, use `sort_by` comparando as contagens.

### Como rodar

```bash
cargo run --release --bin exercicio_contador_palavras -- dados/textos
```

### Solução

A solução está em [`solucao/exercicio_contador_palavras.rs`](./solucao/exercicio_contador_palavras.rs). Só olhe depois de tentar implementar sozinho.

---

## Checklist de aprendizado

Antes de ir para o módulo 01, confira se você consegue:

- [ ] Explicar por que `total_vendas(vendas)` impede de usar `vendas` depois.
- [ ] Dizer a diferença entre `&[T]` e `&mut [T]`.
- [ ] Criar um trait e implementá-lo para dois tipos diferentes.
- [ ] Escrever uma função genérica sobre um trait.
- [ ] Explicar quando usar `thiserror` e quando usar `anyhow`.
- [ ] Propagar erros com `?` em vez de `unwrap`.
- [ ] Escolher entre `rayon` e `tokio` para um cenário novo.
- [ ] Ler e escrever um CSV com normalização.

Se algum item ficou nebuloso, volte ao exemplo correspondente e mexa no código.

---

## Leituras complementares

- [The Rust Book — Understanding Ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [The Rust Book — Traits](https://doc.rust-lang.org/book/ch10-02-traits.html)
- [The Rust Book — Error Handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [The Rust Book — Fearless Concurrency](https://doc.rust-lang.org/book/ch16-00-concurrency.html)
- [Rayon docs](https://docs.rs/rayon)
- [Tokio tutorial](https://tokio.rs/tokio/tutorial)
