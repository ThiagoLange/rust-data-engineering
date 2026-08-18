# CLAUDE.md

Guia para o Claude Code trabalhar neste repositório. Este é um **treinamento de Rust para Engenharia de Dados e IA**, estruturado em 12 módulos progressivos (00 a 11). Os READMEs de cada módulo já existem e definem o que precisa ser implementado — este arquivo define **como** implementar.

## Visão geral do projeto

Repositório educacional público. Cada módulo em `NN-nome-do-modulo/` contém:
- `README.md` — já escrito, contém teoria, lista de exemplos e enunciado do exercício
- `src/` — onde os exemplos de código devem ser implementados
- `Cargo.toml` — a criar por módulo (cada módulo é um crate/workspace independente, não um workspace único)
- Exercício resolvido vive numa **branch separada**, nunca na `main`

Sempre leia o `README.md` do módulo antes de implementar qualquer coisa nele — é a fonte da verdade sobre o que cada exemplo deve demonstrar.

## Ordem de trabalho

Implemente os módulos **em ordem** (00 → 11). Módulos avançados assumem crates e padrões introduzidos em módulos anteriores (ex: o Módulo 7 reusa `hnsw_rs` e o padrão de embeddings duplos que aparecem também no contexto do projeto AI-Lake). Não pule para um módulo avançado sem os anteriores implementados, a menos que explicitamente pedido.

## Convenções de código

- **Edition**: Rust 2021, sempre a stable mais recente.
- **Formatação**: `cargo fmt` antes de qualquer commit. Sem exceções.
- **Lints**: `cargo clippy -- -D warnings` deve passar limpo em todo módulo.
- **Erros**: `anyhow` nos binários de exemplo (simplicidade pedagógica), `thiserror` se o módulo introduzir algo reutilizável como biblioteca. Nunca `unwrap()`/`expect()` fora de testes — isso é um treinamento, o código exemplifica boas práticas.
- **Comentários**: comente o *porquê*, não o *o quê*. Este código é lido por quem está aprendendo — prefira nomes de variáveis claros a comentários óbvios, mas explique decisões não triviais (ex: por que lazy evaluation aqui, por que esse tamanho de buffer).
- **Sem `unsafe`** em nenhum exemplo, a menos que o módulo seja explicitamente sobre isso (não é o caso de nenhum aqui).
- **Async**: use `tokio` com `#[tokio::main]` nos binários que precisam; não misture com `rayon` no mesmo escopo sem justificar no comentário.

## Estrutura de cada módulo

Ao implementar um módulo `NN-nome/`:

1. Crie `Cargo.toml` com as dependências mínimas necessárias (não adicione crates "por via das dúvidas")
2. Cada exemplo do README vira um binário separado em `src/bin/nome_exemplo.rs` (ou `src/main.rs` se o módulo tiver só um exemplo)
3. Adicione um bloco de comentário no topo de cada arquivo de exemplo linkando de volta à seção correspondente do README
4. Dados de exemplo (CSVs, Parquet pequenos, imagens de teste) vão em `NN-nome/dados/` — gere dados sintéticos pequenos (não baixe datasets grandes de terceiros)
5. Se o módulo precisa de infraestrutura local (Kafka, Jaeger), crie `docker-compose.yml` na raiz do módulo
6. Todo exemplo deve rodar com um único comando `cargo run --bin nome` sem setup manual além do que está documentado no README

## Exercícios e branches de solução

- O enunciado do exercício já está no README do módulo — não o reescreva.
- Implemente a solução numa branch `solucoes/NN-nome-do-modulo`, nunca na `main`.
- Antes de criar a branch de solução, confirme que os exemplos principais do módulo (na `main`) já estão completos e passando em `cargo test`.
- Fluxo: `git checkout -b solucoes/NN-nome-do-modulo`, implemente, commit, volte para `main` com `git checkout main`.

## Testes

- Todo exemplo com lógica não trivial (parsers, transformações, agregações) precisa de ao menos 2-3 testes unitários no mesmo arquivo (`#[cfg(test)] mod tests`).
- Módulos que envolvem parsing (01, e qualquer parser binário/customizado) devem ter testes com entradas malformadas, não só o caminho feliz.
- Não é necessário mockar serviços externos (Kafka, APIs de LLM) nos testes unitários — para isso, use os exemplos rodáveis via `docker-compose` como validação manual, documentada no README.

## Dependências: crates específicos por módulo

Ao adicionar dependências, use as versões estáveis mais recentes no momento da implementação (rode `cargo add nome_crate` em vez de fixar versão manualmente no `Cargo.toml`). Os crates de referência por módulo, conforme já definidos nos READMEs:

- **00**: `rayon`, `tokio`, `anyhow`, `thiserror`
- **01**: `serde`, `serde_json`, `csv`, `winnow`
- **02**: `polars`, `datafusion`, `arrow`
- **03**: `parquet`, `deltalake`, `iceberg-rust`, `object_store`
- **04**: `rdkafka`, `tokio-stream`
- **05**: `tonic`, `prost`
- **06**: `linfa`, `candle-core`, `burn`, `ort`
- **07**: `async-openai`, `reqwest`, `hnsw_rs`, `rig-core`, `llama-cpp-rs`
- **08**: `image`, `opencv`, `ort`
- **09**: `plotters`, `egui`
- **10**: `tracing`, `tracing-opentelemetry`, `criterion`, `proptest`
- **11**: combina os crates dos módulos relevantes por projeto

Se algum crate estiver com o ecossistema marcado como 🟡 ou 🔴 no README do módulo, teste a versão antes de comprometer o exemplo a ela — se estiver quebrada ou com API muito instável, documente isso no README do módulo (seção "Estado do ecossistema") em vez de forçar o exemplo a funcionar de forma frágil.

## Chamadas a APIs externas (Módulo 7 — LLM)

- Nunca hardcode API keys. Use variáveis de ambiente (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) lidas via `std::env::var`, documentadas num `.env.example` no módulo.
- Exemplos que chamam APIs pagas devem ter um aviso claro no README/comentário sobre custo, e devem limitar tokens/chamadas por padrão (não rodar loops ilimitados).

## O que NÃO fazer

- Não reescreva ou resuma os READMEs existentes — eles são a especificação. Se algo nos READMEs precisar mudar, avise antes de alterar.
- Não crie um workspace Cargo único para o repositório inteiro — cada módulo é independente, para refletir que alguém pode estudar/rodar só um módulo sem baixar dependências de todos os outros (ex: não forçar quem só quer estudar o Módulo 1 a compilar `opencv` do Módulo 8).
- Não baixe datasets externos grandes — gere dados sintéticos com tamanho didático (milhares de linhas, não milhões, a menos que o exemplo seja especificamente sobre performance em escala).
- Não adicione módulos, pastas ou exemplos além do que está no roadmap do `README.md` raiz sem confirmar antes.

## Comandos úteis

```bash
# validar um módulo antes de considerar pronto
cd NN-nome-do-modulo
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo run --bin nome_do_exemplo

# criar branch de solução de exercício
git checkout -b solucoes/NN-nome-do-modulo
```
