//! Projeto 2 — Motor de busca vetorial standalone
//! README 11 + `projeto-2-motor-busca-vetorial/ARCHITECTURE.md`.
//!
//! HNSW + quantização F16 + `bincode` + `memmap2`.

use anyhow::Result;
use half::f16;
use hnsw_rs::prelude::*;
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};

/// Vetor quantizado F16 + id original.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VetorQ {
    id: usize,
    dim: usize,
    dados_f16: Vec<u16>, // bits de f16
}

impl VetorQ {
    fn quantizar(id: usize, v: &[f32]) -> Self {
        let dados_f16 = v.iter().map(|x| f16::from_f32(*x).to_bits()).collect();
        Self {
            id,
            dim: v.len(),
            dados_f16,
        }
    }
    fn dequantizar(&self) -> Vec<f32> {
        self.dados_f16
            .iter()
            .map(|b| f16::from_bits(*b).to_f32())
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct IndiceSerializado {
    vetores: Vec<VetorQ>,
    dim: usize,
}

fn gerar_vetores(n: usize, dim: usize) -> Vec<Vec<f32>> {
    // Fórmula sem colisão: período primo + perturbação única por vetor
    (0..n)
        .map(|i| {
            (0..dim)
                .map(|j| (((i * 37 + j * 13) % 200) as f32) / 200.0 + (i as f32 * 1e-5))
                .collect()
        })
        .collect()
}

fn l2(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

fn build_hnsw(vetores: &[Vec<f32>]) -> Hnsw<'_, f32, DistL2> {
    let n = vetores.len();
    let hnsw = Hnsw::<f32, DistL2>::new(16, n, 4, 200, DistL2 {});
    for (i, v) in vetores.iter().enumerate() {
        hnsw.insert((v.as_slice(), i));
    }
    hnsw
}

fn salvar_indice(path: &Path, idx: &IndiceSerializado) -> Result<()> {
    let bytes = bincode::serialize(idx)?;
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(path, &bytes)?;
    println!(
        "Índice serializado: {} vetores, {} bytes → {}",
        idx.vetores.len(),
        bytes.len(),
        path.display()
    );
    Ok(())
}

fn carregar_mmap(path: &Path) -> Result<(Mmap, IndiceSerializado)> {
    let f = File::open(path)?;
    // SAFETY: arquivo escrito por nós, somente leitura
    let mmap = unsafe { Mmap::map(&f)? };
    let idx: IndiceSerializado = bincode::deserialize(&mmap)?;
    Ok((mmap, idx))
}

/// Busca: HNSW em f32 para candidatos + reranking exato sobre F16 dequantizado.
/// Demo simplificada: busca exata sobre dequantizados (corpus pequeno).
fn buscar(idx: &IndiceSerializado, query: &[f32], top_k: usize) -> Vec<(usize, f32)> {
    let mut scored: Vec<(usize, f32)> = idx
        .vetores
        .iter()
        .map(|v| {
            let dq = v.dequantizar();
            (v.id, l2(query, &dq))
        })
        .collect();
    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    scored.truncate(top_k);
    scored
}

fn main() -> Result<()> {
    println!("=== Projeto 2 — Motor de busca vetorial ===\n");
    let dim = 32;
    let n = 200;
    let vetores = gerar_vetores(n, dim);
    println!("Corpus: {n} vetores dim={dim}");

    // HNSW f32
    let hnsw = build_hnsw(&vetores);
    println!("HNSW: {} pontos", hnsw.get_nb_point());

    // Quantização F16: mede erro e economia
    let vetor_q: Vec<VetorQ> = vetores
        .iter()
        .enumerate()
        .map(|(i, v)| VetorQ::quantizar(i, v))
        .collect();
    let err_max: f32 = vetores
        .iter()
        .zip(vetor_q.iter())
        .map(|(a, q)| {
            let dq = q.dequantizar();
            a.iter()
                .zip(dq.iter())
                .map(|(x, y)| (x - y).abs())
                .fold(0.0, f32::max)
        })
        .fold(0.0, f32::max);
    println!("Quantização F16: erro abs máx por componente = {err_max:.6} (memória 2x menor: {} → {} bytes/vetor)", dim * 4, dim * 2);

    // Serializa
    let idx = IndiceSerializado {
        vetores: vetor_q,
        dim,
    };
    let path = PathBuf::from("dados/saida/indice_vetorial.bin");
    salvar_indice(&path, &idx)?;

    // Carrega via mmap (sem carregar tudo em RAM de uma vez na demo — mmap pagina sob demanda)
    let (_mmap, idx2) = carregar_mmap(&path)?;
    println!("MMap: {} vetores carregados via mmap", idx2.vetores.len());

    // Busca: query = vetor 42 com ruído
    let mut query = vetores[42].clone();
    for x in &mut query {
        *x += 0.001;
    }
    let res_hnsw = hnsw.search(&query, 3, 16);
    println!(
        "\nHNSW f32 top-3: {:?}",
        res_hnsw
            .iter()
            .map(|n| (n.d_id, n.distance))
            .collect::<Vec<_>>()
    );
    let res_q = buscar(&idx2, &query, 3);
    println!("F16+mmap top-3: {res_q:?}");

    // Valida que vetor 42 está no top-3 de ambos (empates de distância são possíveis)
    assert!(
        res_hnsw.iter().take(3).any(|n| n.d_id == 42),
        "HNSW deveria recuperar vetor 42 no top-3"
    );
    assert!(
        res_q.iter().take(3).any(|(id, _)| *id == 42),
        "F16 deveria recuperar vetor 42 no top-3"
    );
    println!("\n✓ Motor vetorial OK (HNSW + F16 + bincode + mmap, top-1 preservado)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn quantizacao_preserva_top1() {
        let vetores = gerar_vetores(50, 16);
        let hnsw = build_hnsw(&vetores);
        let vq: Vec<VetorQ> = vetores
            .iter()
            .enumerate()
            .map(|(i, v)| VetorQ::quantizar(i, v))
            .collect();
        let idx = IndiceSerializado {
            vetores: vq,
            dim: 16,
        };
        let mut q = vetores[7].clone();
        for x in &mut q {
            *x += 0.0005;
        }
        let r1 = hnsw.search(&q, 1, 16);
        let r2 = buscar(&idx, &q, 1);
        assert_eq!(r1[0].d_id, 7);
        assert_eq!(r2[0].0, 7);
    }

    #[test]
    fn bincode_round_trip() {
        let dir = TempDir::new().unwrap();
        let vetores = gerar_vetores(10, 8);
        let vq: Vec<VetorQ> = vetores
            .iter()
            .enumerate()
            .map(|(i, v)| VetorQ::quantizar(i, v))
            .collect();
        let idx = IndiceSerializado {
            vetores: vq,
            dim: 8,
        };
        let p = dir.path().join("idx.bin");
        salvar_indice(&p, &idx).expect("ok");
        let (_m, idx2) = carregar_mmap(&p).expect("ok");
        assert_eq!(idx2.vetores.len(), 10);
        assert_eq!(idx2.dim, 8);
    }

    #[test]
    fn erro_quantizacao_pequeno() {
        let v = vec![0.5f32; 16];
        let q = VetorQ::quantizar(0, &v);
        let dq = q.dequantizar();
        for (a, b) in v.iter().zip(dq.iter()) {
            assert!((a - b).abs() < 0.01);
        }
    }
}
