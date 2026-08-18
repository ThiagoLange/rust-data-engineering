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

## Exemplo prático 1: Classificador com linfa

`src/linfa_classifier.rs`: treinar um classificador (regressão logística) sobre um dataset Parquet do Módulo 3, avaliar métricas (accuracy, F1), e salvar o modelo.

## Exemplo prático 2: Inferência ONNX

`src/onnx_inference.rs`: carregar um modelo ONNX exportado do PyTorch (fornecido em `models/`) e rodar inferência em batch sobre dados do pipeline, integrando com Polars para pré-processamento.

```bash
cargo run --bin linfa_classifier
cargo run --bin onnx_inference -- --model models/classifier.onnx
```

## Exercício

Compare o tempo de inferência (latência p50/p99) entre um modelo rodando nativamente em `candle` vs o mesmo modelo via `ort`/ONNX, sobre o mesmo batch de dados. Documente os trade-offs observados. Solução em `solucoes/06-machine-learning`.

## Leituras complementares

- [linfa](https://github.com/rust-ml/linfa)
- [candle](https://github.com/huggingface/candle)
- [burn](https://burn.dev/)
- [ort](https://ort.pyke.io/)
