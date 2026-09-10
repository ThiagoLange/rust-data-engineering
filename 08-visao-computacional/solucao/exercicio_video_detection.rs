//! Módulo 08 — Visão Computacional
//! README: seção "Exercício" — Detecção em vídeo + série temporal
//!
//! Combina detecção de objetos frame a frame num vídeo, conta quantos objetos
//! de cada classe aparecem ao longo do tempo e exporta como CSV (conecta com
//! Módulo 1 — parsing CSV). Demonstra pipeline completo: vídeo → detecção → série temporal.

use anyhow::{Context, Result};
use image::{DynamicImage, RgbImage};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct Detection {
    classe: String,
    conf: f32,
}

// Simula detecção — em produção usaria `ort` + YOLO como em 01_object_detection.rs
fn detectar_frame(frame_idx: usize, _img: &DynamicImage) -> Vec<Detection> {
    // Padrão sintético: a cada frame, número de objetos varia
    // frame 0: 2 pessoas, frame 1: 1 pessoa + 1 carro, etc.
    let mut dets = Vec::new();
    // Pessoa aparece em todos os frames, 1-3 por frame
    let n_pessoas = (frame_idx % 3) + 1;
    for i in 0..n_pessoas {
        dets.push(Detection {
            classe: "pessoa".to_string(),
            conf: 0.90 + (i as f32 * 0.02),
        });
    }
    // Carro aparece a cada 2 frames
    if frame_idx.is_multiple_of(2) {
        dets.push(Detection {
            classe: "carro".to_string(),
            conf: 0.85,
        });
    }
    dets
}

fn gerar_frames_sinteticos(dir: &Path, n: usize) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let mut paths = Vec::new();
    for i in 0..n {
        let mut img = RgbImage::new(320, 240);
        for p in img.pixels_mut() {
            *p = image::Rgb([20, 20, 20]);
        }
        let p = dir.join(format!("frame_{i:04}.jpg"));
        img.save(&p)?;
        paths.push(p);
    }
    Ok(paths)
}

fn processar_video(video_dir: &Path, csv_saida: &Path, classe_alvo: &str) -> Result<()> {
    let frames: Vec<PathBuf> = if video_dir.exists() && video_dir.is_dir() {
        let mut v: Vec<PathBuf> = std::fs::read_dir(video_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("jpg"))
            .collect();
        v.sort();
        v
    } else {
        println!(
            "Vídeo não encontrado, gerando 10 frames sintéticos em {}",
            video_dir.display()
        );
        gerar_frames_sinteticos(video_dir, 10)?
    };

    println!(
        "Processando {} frames para classe '{}'",
        frames.len(),
        classe_alvo
    );

    let mut serie: Vec<(usize, usize)> = Vec::new(); // (frame_idx, count)

    for (idx, frame_path) in frames.iter().enumerate() {
        let img =
            image::open(frame_path).with_context(|| format!("abrindo {}", frame_path.display()))?;
        let dets = detectar_frame(idx, &img);
        let count = dets.iter().filter(|d| d.classe == classe_alvo).count();
        println!(
            "  frame {idx}: {} objetos (total dets={}, alvo={count})",
            dets.len(),
            count
        );
        serie.push((idx, count));
    }

    // Exporta CSV série temporal
    let mut wtr = csv::Writer::from_path(csv_saida)
        .with_context(|| format!("criando {}", csv_saida.display()))?;
    wtr.write_record(["frame", "timestamp", "classe", "count"])?;
    for (frame_idx, count) in serie {
        // timestamp simulado: frame * 33ms (30fps)
        let ts_ms = frame_idx * 33;
        wtr.write_record([
            frame_idx.to_string(),
            ts_ms.to_string(),
            classe_alvo.to_string(),
            count.to_string(),
        ])?;
    }
    wtr.flush()?;
    println!("\nSérie temporal salva em {}", csv_saida.display());

    // Também demonstra agregação com Polars-like manual: média por classe
    let total: usize = std::fs::read_to_string(csv_saida)?
        .lines()
        .skip(1)
        .filter_map(|l| l.split(',').nth(3)?.parse::<usize>().ok())
        .sum();
    let media = total as f64 / frames.len() as f64;
    println!("Média de '{}' por frame: {:.2}", classe_alvo, media);

    // Mostra como conectar com Módulo 9 (visualização) — CSV pronto para `plotters`
    println!("Pronto para visualização no Módulo 9 (plotters): frame vs count");

    Ok(())
}

fn main() -> Result<()> {
    let video_dir = Path::new("dados/tmp_video_in");
    let csv_saida = Path::new("dados/saida/serie_temporal_deteccao.csv");
    std::fs::create_dir_all(csv_saida.parent().unwrap())?;

    println!("=== Exercício Vídeo + Detecção — Módulo 08 ===\n");

    // Conta "pessoa" ao longo do tempo
    processar_video(video_dir, csv_saida, "pessoa")?;

    // Também demonstra para "carro"
    let csv_carro = Path::new("dados/saida/serie_temporal_carro.csv");
    processar_video(video_dir, csv_carro, "carro")?;

    println!("\n✓ Exercício concluído — séries temporais exportadas");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detectar_retorna_vetor() {
        let img = DynamicImage::new_rgb8(10, 10);
        let dets = detectar_frame(0, &img);
        assert!(!dets.is_empty());
        assert!(dets.iter().any(|d| d.classe == "pessoa"));
    }

    #[test]
    fn serie_temporal_gerada() {
        let dir_in = TempDir::new().unwrap();
        let dir_out = TempDir::new().unwrap();
        let csv = dir_out.path().join("out.csv");
        gerar_frames_sinteticos(dir_in.path(), 3).expect("ok");
        processar_video(dir_in.path(), &csv, "pessoa").expect("ok");
        assert!(csv.exists());
        let content = std::fs::read_to_string(&csv).expect("ok");
        assert!(content.contains("frame,timestamp,classe,count"));
        assert!(content.lines().count() >= 4); // header + 3 frames
    }

    #[test]
    fn contar_classe_especifica() {
        let dir = TempDir::new().unwrap();
        let csv = dir.path().join("out.csv");
        gerar_frames_sinteticos(dir.path().join("in").as_path(), 5).expect("ok");
        processar_video(&dir.path().join("in"), &csv, "carro").expect("ok");
        let content = std::fs::read_to_string(&csv).expect("ok");
        // Carro aparece a cada 2 frames, então deve ter pelo menos 2 ocorrências em 5 frames
        let counts: Vec<usize> = content
            .lines()
            .skip(1)
            .filter_map(|l| l.split(',').nth(3)?.parse().ok())
            .collect();
        assert_eq!(counts.len(), 5);
        assert!(counts.iter().any(|&c| c > 0));
    }
}
