# Projeto 2 — Motor de busca vetorial standalone — Arquitetura

Motor simples do zero: HNSW + quantização F16 + `bincode` + `memmap2`.

## Etapas

1. **Indexação**: vetores f32 → índice `hnsw_rs` + cópia quantizada F16 (`half::f16`)
   para reduzir memória 2x.
2. **Serialização**: `Vec<f16>` + metadados via `bincode` em arquivo único.
3. **Carregamento mmap**: `memmap2::Mmap` para busca sem carregar tudo em RAM
   (demo: mmap do arquivo serializado + deserialize da região).
4. **Busca**: dequantiza top candidatos F16→f32 e reordena por L2 exata.

## Roteiro incremental

- `etapa-1`: HNSW f32 + busca.
- `etapa-2`: quantização F16 e medida de erro.
- `etapa-3`: bincode + mmap.

## Testes

`cargo test --bin projeto_2_motor_busca`: quantização preserva top-1,
round-trip bincode, mmap carrega.
