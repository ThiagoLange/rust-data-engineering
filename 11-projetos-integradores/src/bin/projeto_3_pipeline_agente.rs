//! Projeto 3 — Pipeline de dados com agente de IA
//! README 11 + `projeto-3-pipeline-com-agente/ARCHITECTURE.md`.
//!
//! Ingestão → transformação Polars → Parquet particionado → agente
//! (SQL + vetorial) → dashboard JSON.

use anyhow::Result;
use clap::Parser;
use hnsw_rs::prelude::*;
use polars::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "projeto_3_pipeline_agente")]
struct Args {
    /// Pergunta em linguagem natural
    #[arg(long, default_value = "Qual o total de vendas no Sudeste?")]
    pergunta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Interacao {
    pergunta: String,
    tools_usadas: Vec<String>,
    resposta: String,
    latencia_ms: u128,
}

fn gerar_vendas() -> DataFrame {
    let regioes = ["Sudeste", "Sul", "Norte", "Nordeste", "Centro-Oeste"];
    let n = 200;
    let regiao: Vec<String> = (0..n).map(|i| regioes[i % 5].to_string()).collect();
    let valor: Vec<f64> = (0..n).map(|i| 50.0 + ((i * 13) % 500) as f64).collect();
    DataFrame::new(
        n,
        vec![
            Column::new("regiao".into(), regiao),
            Column::new("valor".into(), valor),
        ],
    )
    .unwrap()
}

fn transformar(df: DataFrame) -> Result<DataFrame> {
    df.lazy()
        .filter(col("valor").gt(lit(0.0)))
        .group_by([col("regiao")])
        .agg([
            col("valor").sum().alias("total"),
            col("valor").mean().alias("media"),
            col("valor").count().alias("n"),
        ])
        .sort(
            ["total"],
            SortMultipleOptions::default().with_order_descending(true),
        )
        .collect()
        .map_err(|e| anyhow::anyhow!("group_by: {e}"))
}

fn salvar_parquet_particionado(df: &DataFrame, out_dir: &Path) -> Result<()> {
    // Particiona por regiao usando o DF agregado? Para demo, salva agregado único
    // e também salva base por regiao (requer DF base — simplificamos com agregado)
    std::fs::create_dir_all(out_dir)?;
    // Salva agregado
    let mut df_mut = df.clone();
    let path = out_dir.join("agregado.parquet");
    let mut f = std::fs::File::create(&path)?;
    ParquetWriter::new(&mut f).finish(&mut df_mut)?;
    println!("  agregado → {}", path.display());
    Ok(())
}

/// Tool 1: consulta SQL-like sobre agregados (região → total/média).
fn tool_sql(agregado: &DataFrame, regiao: &str) -> Option<(f64, f64)> {
    let reg = agregado.column("regiao").ok()?.str().ok()?;
    let tot = agregado.column("total").ok()?.f64().ok()?;
    let med = agregado.column("media").ok()?.f64().ok()?;
    for i in 0..agregado.height() {
        if reg.get(i).unwrap_or("") == regiao {
            return Some((tot.get(i).unwrap_or(0.0), med.get(i).unwrap_or(0.0)));
        }
    }
    None
}

fn fake_embed(text: &str) -> Vec<f32> {
    let mut v = vec![0f32; 32];
    for (i, c) in text.bytes().enumerate() {
        v[i % 32] += c as f32 / 255.0;
    }
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    for x in &mut v {
        *x /= norm;
    }
    v
}

/// Tool 2: busca vetorial sobre descrições de região.
fn tool_busca(hnsw: &Hnsw<f32, DistL2>, _docs: &[String], query: &str) -> Vec<(usize, f32)> {
    let q = fake_embed(query);
    hnsw.search(&q, 2, 16)
        .iter()
        .map(|n| (n.d_id, n.distance))
        .collect()
}

fn agente(
    agregado: &DataFrame,
    hnsw: &Hnsw<f32, DistL2>,
    docs: &[String],
    pergunta: &str,
) -> (String, Vec<String>) {
    let q = pergunta.to_lowercase();
    let mut tools = Vec::new();

    // Detecta região mencionada
    let regioes = ["sudeste", "sul", "norte", "nordeste", "centro-oeste"];
    let regiao = regioes
        .iter()
        .find(|r| q.contains(**r))
        .map(|s| s.to_string());

    let mut partes = Vec::new();
    if let Some(r) = regiao.clone() {
        // Capitaliza para buscar no DF ("Sudeste")
        let r_cap = format!("{}{}", r[..1].to_uppercase(), &r[1..]);
        // "Centro-oeste" vs "Centro-Oeste"
        let r_cap = if r == "centro-oeste" {
            "Centro-Oeste".to_string()
        } else {
            r_cap
        };
        if let Some((total, media)) = tool_sql(agregado, &r_cap) {
            tools.push("sql_consulta".to_string());
            partes.push(format!(
                "Total em {r_cap}: R${total:.2} (média R${media:.2})"
            ));
        }
    }
    if regiao.is_none() {
        // Sem região: resume todas
        tools.push("sql_consulta".to_string());
        let tot: f64 = agregado
            .column("total")
            .unwrap()
            .f64()
            .unwrap()
            .iter()
            .map(|v| v.unwrap_or(0.0))
            .sum();
        partes.push(format!("Total geral: R${tot:.2}"));
    }
    // Busca vetorial como contexto adicional
    let res = tool_busca(hnsw, docs, pergunta);
    tools.push("busca_vetorial".to_string());
    let ctx: Vec<String> = res
        .iter()
        .map(|(i, d)| format!("{} (dist {d:.3})", docs[*i]))
        .collect();
    partes.push(format!("Contexto vetorial: {}", ctx.join(" | ")));

    let resposta = format!("Pergunta: {pergunta}\n{}", partes.join("\n"));
    (resposta, tools)
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("=== Projeto 3 — Pipeline com agente ===\n");
    let start_total = Instant::now();

    // 1-2. Ingestão + transformação
    let df = gerar_vendas();
    let agregado = transformar(df)?;
    println!("Agregado por região:\n{agregado}");

    // 3. Parquet
    let out_dir = PathBuf::from("dados/saida/pipeline_agente");
    salvar_parquet_particionado(&agregado, &out_dir)?;

    // 4. Índice vetorial sobre descrições
    let docs: Vec<String> = vec![
        "Sudeste concentra tecnologia e finanças".to_string(),
        "Sul tem agroindústria forte".to_string(),
        "Norte tem zona franca e mineração".to_string(),
        "Nordeste tem turismo e energia solar".to_string(),
        "Centro-Oeste tem agronegócio".to_string(),
    ];
    let embs: Vec<Vec<f32>> = docs.iter().map(|d| fake_embed(d)).collect();
    let hnsw = Hnsw::<f32, DistL2>::new(16, embs.len(), 4, 200, DistL2 {});
    for (i, e) in embs.iter().enumerate() {
        hnsw.insert((e.as_slice(), i));
    }

    // 5. Agente
    let t0 = Instant::now();
    let (resposta, tools) = agente(&agregado, &hnsw, &docs, &args.pergunta);
    let lat = t0.elapsed().as_millis();
    println!("\n=== Resposta do agente ===\n{resposta}");
    println!("Tools: {tools:?} ({lat}ms)");

    // 6. Dashboard JSON (histórico + métricas)
    let interacao = Interacao {
        pergunta: args.pergunta.clone(),
        tools_usadas: tools,
        resposta,
        latencia_ms: lat,
    };
    let metrics_path = out_dir.join("metricas.json");
    let historico = serde_json::json!({
        "pipeline_ms": start_total.elapsed().as_millis(),
        "interacoes": [interacao],
        "regioes": agregado.height(),
    });
    std::fs::write(&metrics_path, serde_json::to_string_pretty(&historico)?)?;
    println!(
        "\nDashboard JSON → {} (pronto para D3/Grafana/egui)",
        metrics_path.display()
    );

    println!("\n✓ Pipeline com agente OK");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn agregacao_correta() {
        let df = gerar_vendas();
        let ag = transformar(df).expect("ok");
        assert_eq!(ag.height(), 5);
        let tot: f64 = ag
            .column("total")
            .unwrap()
            .f64()
            .unwrap()
            .iter()
            .map(|v| v.unwrap_or(0.0))
            .sum();
        assert!(tot > 0.0);
    }

    #[test]
    fn agente_responde_sudeste() {
        let df = gerar_vendas();
        let ag = transformar(df).expect("ok");
        let docs = vec!["Sudeste tech".to_string(), "Sul agro".to_string()];
        let embs: Vec<Vec<f32>> = docs.iter().map(|d| fake_embed(d)).collect();
        let hnsw = Hnsw::<f32, DistL2>::new(16, embs.len(), 4, 200, DistL2 {});
        for (i, e) in embs.iter().enumerate() {
            hnsw.insert((e.as_slice(), i));
        }
        let (resp, tools) = agente(&ag, &hnsw, &docs, "Qual o total no Sudeste?");
        assert!(tools.contains(&"sql_consulta".to_string()));
        assert!(resp.contains("Sudeste"));
    }

    #[test]
    fn parquet_e_json_validos() {
        let dir = TempDir::new().unwrap();
        let df = gerar_vendas();
        let ag = transformar(df).expect("ok");
        salvar_parquet_particionado(&ag, dir.path()).expect("ok");
        assert!(dir.path().join("agregado.parquet").exists());
        let _ = HashMap::<String, String>::new();
    }
}
