# 08 — Visão Computacional

**Nível:** Avançado
**Estado do ecossistema:** 🟢 image crate · 🟡 opencv-rust (bindings maduros, mas dependem do OpenCV instalado no sistema) · 🔴 kornia-rs (nicho, em desenvolvimento ativo)

## Conteúdo

1. **image crate** — manipulação básica: resize, crop, conversão de formato, filtros simples. Sem dependências externas, 100% Rust
2. **opencv-rust** — bindings completos do OpenCV (requer libopencv instalada no sistema). Acesso a toda a superfície de OpenCV: detecção de features, tracking, calibração de câmera
3. **Inferência de modelos de visão via ONNX** — rodar YOLO, ResNet e afins usando `ort`, mesmo padrão do Módulo 6 aplicado a imagens
4. **kornia-rs** — tentativa de visão computacional nativa em Rust (sem bindings C++), inspirada no Kornia (PyTorch). Vale conhecer, mas ainda imatura para produção

## Exemplos práticos

| Binário | Arquivo | O que demonstra |
|---------|---------|----------------|
| `01_object_detection` | `src/bin/01_object_detection.rs` | YOLO ONNX via `ort` (`Session::builder`, `preprocess` CHW), fallback sintético se `models/yolo.onnx` ausente, `image`/`imageproc::drawing::draw_hollow_rect_mut`, salva `dados/saida/deteccao_saida.jpg` |
| `02_video_processing` | `src/bin/02_video_processing.rs` | Vídeo frame a frame — `opencv` (`VideoCapture`/`Canny`) se `--features opencv` e `.mp4` presente, senão fallback `image`+`imageproc::edges::canny` com frames sintéticos em `dados/tmp_frames_in` → `dados/saida/video_frames_out` |

```bash
cargo run --bin 01_object_detection -- --image dados/imagens/entrada.jpg --model models/yolo.onnx
cargo run --bin 02_video_processing -- --input dados/video_entrada.mp4 --output dados/saida/video_processado.mp4
# fallback sem OpenCV (usa imageproc):
cargo run --bin 02_video_processing
# com OpenCV (requer libopencv):
cargo run --features opencv --bin 02_video_processing -- --input video.mp4 --output out.mp4
```

## Exercício

Combine os dois exemplos: rode detecção de objetos frame a frame num vídeo, contando quantos objetos de uma classe específica aparecem ao longo do tempo, e exporte os resultados como uma série temporal (CSV) — conectando com o que foi aprendido no Módulo 1. Solução em `solucao/exercicio_video_detection.rs` (bin `exercicio_video_detection`) — `detectar_frame` simulado + `processar_video` → `dados/saida/serie_temporal_*.csv` (pronto para `plotters` no Módulo 9).

```bash
cargo run --bin exercicio_video_detection
```

## Leituras complementares

- [image crate](https://docs.rs/image)
- [opencv-rust](https://github.com/twistedfall/opencv-rust)
- [kornia-rs](https://github.com/kornia/kornia-rs)
