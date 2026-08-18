# 08 — Visão Computacional

**Nível:** Avançado
**Estado do ecossistema:** 🟢 image crate · 🟡 opencv-rust (bindings maduros, mas dependem do OpenCV instalado no sistema) · 🔴 kornia-rs (nicho, em desenvolvimento ativo)

## Conteúdo

1. **image crate** — manipulação básica: resize, crop, conversão de formato, filtros simples. Sem dependências externas, 100% Rust
2. **opencv-rust** — bindings completos do OpenCV (requer libopencv instalada no sistema). Acesso a toda a superfície de OpenCV: detecção de features, tracking, calibração de câmera
3. **Inferência de modelos de visão via ONNX** — rodar YOLO, ResNet e afins usando `ort`, mesmo padrão do Módulo 6 aplicado a imagens
4. **kornia-rs** — tentativa de visão computacional nativa em Rust (sem bindings C++), inspirada no Kornia (PyTorch). Vale conhecer, mas ainda imatura para produção

## Exemplo prático 1: Pipeline de detecção de objetos

`src/object_detection.rs`: carregar um modelo YOLO exportado em ONNX, rodar inferência sobre uma imagem, decodificar as bounding boxes e desenhar o resultado usando a `image` crate.

## Exemplo prático 2: Processamento de vídeo frame a frame

`src/video_processing.rs`: usando `opencv-rust`, ler um vídeo, aplicar uma transformação por frame (ex: detecção de bordas), e escrever o vídeo processado de volta.

```bash
cargo run --bin object_detection -- --image entrada.jpg --model models/yolo.onnx
cargo run --bin video_processing -- --input video.mp4 --output processado.mp4
```

## Exercício

Combine os dois exemplos: rode detecção de objetos frame a frame num vídeo, contando quantos objetos de uma classe específica aparecem ao longo do tempo, e exporte os resultados como uma série temporal (CSV) — conectando com o que foi aprendido no Módulo 1. Solução em `solucoes/08-visao-computacional`.

## Leituras complementares

- [image crate](https://docs.rs/image)
- [opencv-rust](https://github.com/twistedfall/opencv-rust)
- [kornia-rs](https://github.com/kornia/kornia-rs)
