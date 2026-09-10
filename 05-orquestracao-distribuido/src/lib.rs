//! Módulo 05 — Orquestração: sharding e proto compartilhado.

pub mod orchestrator {
    tonic::include_proto!("orchestrator");
}

use std::collections::{hash_map::DefaultHasher, HashMap};
use std::hash::{Hash, Hasher};

/// Calcula o shard por hash do nome do arquivo.
/// Determinístico — mesmo arquivo sempre cai no mesmo shard.
pub fn shard_for_file(file: &str, num_shards: usize) -> usize {
    if num_shards == 0 {
        return 0;
    }
    let mut hasher = DefaultHasher::new();
    file.hash(&mut hasher);
    (hasher.finish() as usize) % num_shards
}

/// Distribui `files` entre `num_shards` via hash.
/// Retorna mapa shard_id → lista de arquivos.
pub fn shard_files(files: Vec<String>, num_shards: usize) -> HashMap<usize, Vec<String>> {
    let mut shards: HashMap<usize, Vec<String>> = HashMap::new();
    for file in files {
        let shard = shard_for_file(&file, num_shards);
        shards.entry(shard).or_default().push(file);
    }
    shards
}

/// Gera lista sintética de arquivos Parquet para demo.
pub fn gerar_lista_parquet(n: usize) -> Vec<String> {
    (0..n)
        .map(|i| format!("dados/parquet/part-{i:04}.parquet"))
        .collect()
}

/// Hash alternativo — range partitioning por prefixo (ex: data).
/// Útil quando a chave tem ordenação (ex: `date=2024-01-01`).
pub fn range_shard(file: &str, boundaries: &[String]) -> usize {
    for (i, b) in boundaries.iter().enumerate() {
        if file < b.as_str() {
            return i;
        }
    }
    boundaries.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shard_deterministico() {
        let s1 = shard_for_file("dados/parquet/part-0001.parquet", 3);
        let s2 = shard_for_file("dados/parquet/part-0001.parquet", 3);
        assert_eq!(s1, s2);
    }

    #[test]
    fn shard_distribui_todos_arquivos() {
        let files = gerar_lista_parquet(12);
        let shards = shard_files(files.clone(), 3);
        let total: usize = shards.values().map(|v| v.len()).sum();
        assert_eq!(total, 12);
    }

    #[test]
    fn shard_balanceado() {
        let files = gerar_lista_parquet(100);
        let shards = shard_files(files, 3);
        // Cada shard deve ter ~33 arquivos (±15)
        for v in shards.values() {
            assert!(v.len() >= 20 && v.len() <= 50, "desbalanceado: {}", v.len());
        }
    }

    #[test]
    fn range_shard_funciona() {
        let boundaries = vec!["m".to_string(), "t".to_string()];
        assert_eq!(range_shard("a", &boundaries), 0);
        assert_eq!(range_shard("n", &boundaries), 1);
        assert_eq!(range_shard("z", &boundaries), 2);
    }
}
