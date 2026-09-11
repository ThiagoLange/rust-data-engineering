# 13 — Contratos e Qualidade de Dados

**Nível:** Intermediário
**Pré-requisito:** módulos 01–03

## Por que este módulo?

Pipelines de dados falham silenciosamente quando o schema muda, tipos divergem ou dados inválidos chegam sem alerta. Contratos de dados (schema registry, validação, quality checks) evitam isso. Este módulo mostra como implementar esses controles em Rust.

## Estado do ecossistema

🟡 **Em maturação**: crates como `datafusion` e `polars` têm boas APIs de schema, mas não há crate de registry nativo. Usamos `serde` + validadores manuais como padrão.

## Exemplos

| Binário | O que demonstra |
|---------|-----------------|
| `01_schema_registry` | Registry de schemas (Arrow/Polars) com versionamento e compatibilidade check |
| `02_data_quality` | Checks de qualidade: nulos, ranges, unicidade, distribuição (estilo Great Expectations) |

```bash
cargo run --bin 01_schema_registry
cargo run --bin 02_data_quality
cargo run --bin exercicio_pipeline_estrict
cargo test
```

## O que implementar

1. **`src/bin/01_schema_registry.rs`** — Registry em memória: registrar schema v1, verificar compatibilidade com v2 (breaking change), serializar via Arrow IPC
2. **`src/bin/02_data_quality.rs`** — Suite de checks: nulos, ranges, unicidade, distribuição, drift detection (KS-test-like)
3. **`solucao/exercicio_pipeline_estrict.rs`** — Pipeline que rejeita dados que violam o schema (ex: tipo errado em coluna) e loga o reason

## Convenções

- Usar `arrow-schema` para definir schemas canônicos
- `thiserror` para errors customizados de validação
- Nunca `unwrap()` em validações — converter para erro descritivo