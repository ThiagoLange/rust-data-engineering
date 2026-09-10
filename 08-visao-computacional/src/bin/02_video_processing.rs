//! Módulo 08 — Visão Computacional
//! README: seção "Exemplo prático 2: Processamento de vídeo frame a frame"
//!
//! Lê vídeo via `opencv` (se feature `opencv` habilitada) ou simula com
//! sequência de imagens, aplica transformação por frame (canny) e escreve
//! vídeo/imagens processadas. Fallback sem OpenCV usa `image`/`imageproc`.

use anyhow::{Context, Result};
use clap::Parser;
use image::{DynamicImage, RgbImage};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "video_processing")]
struct Args {
    /// Vídeo de entrada (ou diretório com frames)
    #[arg(long, default_value = "dados/video_entrada.mp4")]
    input: PathBuf,

    /// Vídeo/saída
    #[arg(long, default_value = "dados/saida/video_processado.mp4")]
    output: PathBuf,

    /// Número de frames sintéticos se input não existir
    #[arg(long, default_value_t = 10)]
    frames: usize,
}

fn aplicar_canny(img: &DynamicImage) -> RgbImage {
    // Converte para grayscale e aplica Canny via imageproc
    let gray = img.to_luma8();
    // imageproc::edges::canny espera Image<Luma<u8>, _> com thresholds
    let edges = imageproc::edges::canny(&gray, 50.0, 100.0);
    // Converte de volta para RGB para salvar como vídeo simulado (frames coloridos)
    let mut rgb = RgbImage::new(edges.width(), edges.height());
    for (x, y, pixel) in edges.enumerate_pixels() {
        let v = pixel[0];
        rgb.put_pixel(x, y, image::Rgb([v, v, v]));
    }
    rgb
}

fn gerar_frames_sinteticos(dir: &Path, n: usize) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let mut paths = Vec::new();
    for i in 0..n {
        let mut img = RgbImage::new(320, 240);
        // Fundo + retângulo móvel
        for p in img.pixels_mut() {
            *p = image::Rgb([20, 20, 20]);
        }
        let x = (i * 20) % 200;
        for dx in 0..80 {
            for dy in 0..60 {
                img.put_pixel(x as u32 + dx, 50 + dy, image::Rgb([200, 50, 50]));
            }
        }
        let p = dir.join(format!("frame_{i:04}.jpg"));
        img.save(&p)?;
        paths.push(p);
    }
    Ok(paths)
}

#[cfg(feature = "opencv")]
mod opencv_impl {
    use super::*;
    use anyhow::Result;
    use opencv::{core, imgproc, prelude::*, videoio};

    pub fn processar_video_opencv(input: &Path, output: &Path) -> Result<()> {
        let mut cap = videoio::VideoCapture::from_file(input.to_str().unwrap(), videoio::CAP_ANY)
            .context("abrindo video com opencv")?;
        if !cap.is_opened()? {
            anyhow::bail!("não foi possível abrir {}", input.display());
        }

        let fps = cap.get(videoio::CAP_PROP_FPS)?;
        let width = cap.get(videoio::CAP_PROP_FRAME_WIDTH)? as i32;
        let height = cap.get(videoio::CAP_PROP_FRAME_HEIGHT)? as i32;
        let fourcc = videoio::VideoWriter::fourcc('m', 'p', '4', 'v')?;

        let mut writer = videoio::VideoWriter::new(
            output.to_str().unwrap(),
            fourcc,
            fps.max(25.0),
            core::Size::new(width, height),
            true,
        )
        .context("criando writer")?;

        let mut frame = core::Mat::default();
        let mut gray = core::Mat::default();
        let mut edges = core::Mat::default();
        let mut processed = 0;

        while cap.read(&mut frame)? {
            if frame.empty()? {
                break;
            }
            imgproc::cvt_color(&frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
            imgproc::canny(&gray, &mut edges, 50.0, 100.0, 3, false)?;
            // Converte de volta para BGR para writer
            let mut edges_bgr = core::Mat::default();
            imgproc::cvt_color(&edges, &mut edges_bgr, imgproc::COLOR_GRAY2BGR, 0)?;
            writer.write(&edges_bgr)?;
            processed += 1;
        }

        println!(
            "OpenCV: {processed} frames processados → {}",
            output.display()
        );
        Ok(())
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("=== Processamento de Vídeo — Módulo 08 ===\n");
    println!("Input: {}", args.input.display());
    println!("Output: {}", args.output.display());

    #[cfg(feature = "opencv")]
    {
        if args.input.exists() && args.input.extension().and_then(|s| s.to_str()) == Some("mp4") {
            println!("Usando OpenCV (feature habilitada) para vídeo real");
            return opencv_impl::processar_video_opencv(&args.input, &args.output);
        } else {
            println!(
                "Input não é .mp4 ou não existe — caindo para fallback com imagens sintéticas"
            );
        }
    }

    #[cfg(not(feature = "opencv"))]
    println!(
        "Feature `opencv` não habilitada — usando fallback `image` + `imageproc` (sem libopencv)"
    );

    // Fallback: gera frames sintéticos e processa com imageproc
    let tmp_in = Path::new("dados/tmp_frames_in");
    let tmp_out = Path::new("dados/saida/video_frames_out");
    let frames = if args.input.exists() && args.input.is_dir() {
        // Se input for diretório com imagens, lista
        std::fs::read_dir(&args.input)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("jpg"))
            .collect::<Vec<_>>()
    } else {
        println!(
            "Gerando {} frames sintéticos em {}",
            args.frames,
            tmp_in.display()
        );
        gerar_frames_sinteticos(tmp_in, args.frames)?
    };

    std::fs::create_dir_all(tmp_out)?;
    std::fs::create_dir_all(args.output.parent().unwrap_or(Path::new("dados/saida")))?;

    for (i, frame_path) in frames.iter().enumerate() {
        let img =
            image::open(frame_path).with_context(|| format!("abrindo {}", frame_path.display()))?;
        let processed = aplicar_canny(&img);
        let out_path = tmp_out.join(format!("frame_{i:04}_edges.jpg"));
        processed.save(&out_path)?;
        if i % 5 == 0 {
            println!(
                "  frame {i}: {} → {}",
                frame_path.display(),
                out_path.display()
            );
        }
    }

    // Também salva um "vídeo" simulado como diretório de frames + log
    println!(
        "\nVídeo processado como sequência de frames em {}",
        tmp_out.display()
    );
    println!("Para vídeo real .mp4, habilite feature `opencv` e forneça .mp4:");
    println!("  cargo run --features opencv --bin 02_video_processing -- --input video.mp4 --output out.mp4");
    println!("✓ Processamento concluído");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn canny_nao_panica() {
        let img = DynamicImage::new_rgb8(32, 32);
        let out = aplicar_canny(&img);
        assert_eq!(out.dimensions(), (32, 32));
    }

    #[test]
    fn gerar_e_processar_frames() {
        let dir_in = TempDir::new().unwrap();
        let dir_out = TempDir::new().unwrap();
        let frames = gerar_frames_sinteticos(dir_in.path(), 3).expect("ok");
        assert_eq!(frames.len(), 3);
        for p in frames {
            let img = image::open(p).expect("ok");
            let out = aplicar_canny(&img);
            assert_eq!(out.dimensions(), (320, 240));
        }
        let _ = dir_out;
    }

    #[test]
    fn gerar_frames_sinteticos_cria_arquivos() {
        let dir = TempDir::new().unwrap();
        let frames = gerar_frames_sinteticos(dir.path(), 2).expect("ok");
        assert_eq!(frames.len(), 2);
        for p in frames {
            assert!(p.exists());
        }
    }
}
