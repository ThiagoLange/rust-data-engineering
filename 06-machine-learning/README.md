# 06 — Machine Learning

**Nível:** Avançado
**Estado do ecossistema:** 🟡 linfa · 🟡 candle · 🟡 burn · 🟢 ort (ONNX Runtime bindings)

## Conteúdo

1. **linfa** — o mais próximo de um "scikit-learn" em Rust: regressão linear/logística, k-means, SVM, PCA. API mais limitada que scikit-learn, mas sólida para os algoritmos que cobre
2. **candle** (Hugging Face) — tensores e redes neurais nativas em Rust, com backends CPU/CUDA/Metal. Focado em inferência eficiente, ganhando tração para rodar modelos localmente
3. **burn** — framework de deep learning mais completo, com backends intercambiáveis (CPU, GPU via WGPU, tch/LibTorch) e treino, não só inferência
4. **ort** — bindings para ONNX Runtime, permitindo rodar modelos treinados em PyTorch/TensorFlow/scikit-learn dentro de um pipeline Rust sem reescrever nada

## Estratégia recomendada

Para a maioria dos casos de engenharia de dados com ML embutido: treine em Python (ecossistema mais maduro), exporte para ONNX, e sirva a inferência em Rust via `ort` — combina produtividade de treino com performance de inferência. Use `linfa`/`candle`/`burn` diretamente quando quiser um pipeline 100% Rust, sem dependência de Python em produção.

## Exemplos práticos

| Binário | Arquivo | O que demonstra |
|---------|---------|----------------|
| `01_linfa_classifier` | `src/bin/01_linfa_classifier.rs` | **Classificação logística** com `linfa` sobre Parquet do Módulo 3 (`quantidade` + `preco_unitario` → `is_high_value`); `split_with_ratio`, `confusion_matrix`, `accuracy`/`F1`/`MCC`, salva `models/linfa_model.json` |
| `02_regressao_linear` | `src/bin/02_regressao_linear.rs` | **Regressão linear** com `linfa-linear` — sintético `y=2.5x+3` e vendas (`quantidade → preco_unitario`); métricas `RMSE`/`R²`, salva `models/linear_model.json` |
| `03_onnx_inference` | `src/bin/03_onnx_inference.rs` | **Classificação via ONNX** — `ort` 2.0 (`Session::builder`, `Tensor::from_array`, `inputs!`) + `Polars` para pré-processamento; fallback sintético se `models/classifier.onnx` não existir; batch 32 |

```bash
cargo run --bin 01_linfa_classifier
cargo run --bin 02_regressao_linear
cargo run --bin 03_onnx_inference -- --model models/classifier.onnx --batch-size 32
```

## Exercício

Compare o tempo de inferência (latência p50/p99) entre um modelo rodando nativamente em `candle` (`candle-core` + `candle-nn`, `Device::Cpu`, `sigmoid`) vs o mesmo modelo via `ort`/ONNX (simulado com `ndarray`), sobre o mesmo batch de dados. Documente os trade-offs observados. Solução em `solucao/exercicio_candle_vs_ort.rs` (bin `exercicio_candle_vs_ort`) — 200 iterações, `percentil`, `max_diff <1e-5`.

```bash
cargo run --bin exercicio_candle_vs_ort
```

## Leituras complementares

- [linfa](https://github.com/rust-ml/linfa)
- [candle](https://github.com/huggingface/candle)
- [burn](https://burn.dev/)
- [ort](https://ort.pyke.io/)
