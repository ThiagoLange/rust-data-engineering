//! Módulo 03 — Armazenamento e Formatos
//! README: seção "Conteúdo", item 2 — Avro
//!
//! Avro é formato orientado a linha com schema evolution, comum em Kafka.
//! Este exemplo escreve o CSV de vendas em Avro (com schema explícito) e lê
//! de volta, validando round-trip e medindo tamanho com diferentes codecs.

use anyhow::{Context, Result};
use apache_avro::reader::datum::GenericDatumReader;
use apache_avro::writer::datum::GenericDatumWriter;
use apache_avro::{from_value, types::Value, Codec, DeflateSettings, Reader, Schema, Writer};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

/// Registro de venda — espelha `dados/vendas.csv`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Venda {
    id: i64,
    data: String,
    categoria: String,
    produto: String,
    quantidade: i32,
    preco_unitario: f64,
    regiao: String,
    cliente_id: i64,
}

const SCHEMA_JSON: &str = r#"
{
  "type": "record",
  "name": "Venda",
  "fields": [
    {"name": "id", "type": "long"},
    {"name": "data", "type": "string"},
    {"name": "categoria", "type": "string"},
    {"name": "produto", "type": "string"},
    {"name": "quantidade", "type": "int"},
    {"name": "preco_unitario", "type": "double"},
    {"name": "regiao", "type": "string"},
    {"name": "cliente_id", "type": "long"}
  ]
}
"#;

fn schema() -> Result<Schema> {
    Schema::parse_str(SCHEMA_JSON).context("parse do schema Avro")
}

/// Lê as primeiras `limite` linhas do CSV para `Vec<Venda>`.
fn ler_vendas(caminho: &Path, limite: Option<usize>) -> Result<Vec<Venda>> {
    let file = File::open(caminho).with_context(|| format!("abrindo {}", caminho.display()))?;
    let mut rdr = csv::Reader::from_reader(file);
    let mut vendas = Vec::new();
    for result in rdr.deserialize() {
        let venda: Venda = result.context("desserializando linha CSV")?;
        vendas.push(venda);
        if let Some(n) = limite {
            if vendas.len() >= n {
                break;
            }
        }
    }
    Ok(vendas)
}

fn escrever_avro(caminho: &Path, vendas: &[Venda], codec: Codec) -> Result<usize> {
    let schema = schema()?;
    let file = File::create(caminho).with_context(|| format!("criando {}", caminho.display()))?;
    let mut writer = Writer::with_codec(&schema, file, codec).context("criando Writer Avro")?;

    for venda in vendas {
        writer.append_ser(venda).context("append Avro")?;
    }
    writer.flush().context("flush Avro")?;

    Ok(std::fs::metadata(caminho)
        .with_context(|| format!("metadata {}", caminho.display()))?
        .len() as usize)
}

fn ler_avro(caminho: &Path) -> Result<Vec<Venda>> {
    let file = File::open(caminho).with_context(|| format!("abrindo {}", caminho.display()))?;
    let reader = Reader::new(file).context("criando Reader Avro")?;
    let mut vendas = Vec::new();
    for value in reader {
        let value = value.context("lendo valor Avro")?;
        let venda: Venda = from_value(&value).context("convertendo Value -> Venda")?;
        vendas.push(venda);
    }
    Ok(vendas)
}

fn main() -> Result<()> {
    let entrada = Path::new("dados/vendas.csv");
    let saida_dir = Path::new("dados/saida/avro");
    let saida_padrao = saida_dir.join("vendas.avro");
    let saida_deflate = saida_dir.join("vendas_deflate.avro");

    let saida_parent = saida_padrao
        .parent()
        .ok_or_else(|| anyhow::anyhow!("caminho sem parent: {}", saida_padrao.display()))?;
    std::fs::create_dir_all(saida_parent).context("criando diretório de saída")?;

    let vendas = ler_vendas(entrada, Some(200))?;
    println!("Lidas {} vendas do CSV", vendas.len());

    // Escrita sem compressão (Null) vs Deflate — compara tamanhos
    let tam_null = escrever_avro(&saida_padrao, &vendas, Codec::Null)?;
    let tam_deflate = escrever_avro(
        &saida_deflate,
        &vendas,
        Codec::Deflate(DeflateSettings::default()),
    )?;
    println!(
        "Avro null    : {tam_null} bytes -> {}",
        saida_padrao.display()
    );
    println!(
        "Avro deflate : {tam_deflate} bytes -> {}",
        saida_deflate.display()
    );

    let lidas = ler_avro(&saida_padrao)?;
    println!("Lidas {} vendas do Avro", lidas.len());
    assert_eq!(vendas.len(), lidas.len());

    // Valida round-trip de um registro
    if let Some(primeira) = vendas.first() {
        assert_eq!(primeira, &lidas[0]);
        println!("Round-trip OK para id={}", primeira.id);
    }

    println!("\nSchema Avro usado:\n{SCHEMA_JSON}");

    // Demonstra datum único (útil em Kafka: valor serializado sem header de arquivo)
    let schema_avro = schema()?;
    let value = apache_avro::to_value(vendas[0].clone()).context("convertendo para Value")?;
    let buf = GenericDatumWriter::builder(&schema_avro)
        .build()
        .context("builder datum writer")?
        .write_value_to_vec(value.clone())
        .context("serializando datum")?;
    let decoded_value = GenericDatumReader::builder(&schema_avro)
        .build()
        .context("builder datum reader")?
        .read_value(&mut buf.as_slice())
        .context("desserializando datum")?;
    let decoded: Venda = from_value(&decoded_value).context("Value -> Venda")?;
    assert_eq!(vendas[0], decoded);
    println!("Datum Avro (single record) OK: {} bytes", buf.len());

    // Evita warning de import não usado
    let _ = Value::Null;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn round_trip_avro() {
        let dir = env::temp_dir().join("avro_round_trip");
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("test.avro");
        let vendas = ler_vendas(Path::new("dados/vendas.csv"), Some(10)).expect("ok");
        escrever_avro(&out, &vendas, Codec::Null).expect("ok");
        let lidas = ler_avro(&out).expect("ok");
        assert_eq!(vendas, lidas);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn avro_deflate_menor_que_null() {
        let dir = env::temp_dir().join("avro_compress");
        std::fs::create_dir_all(&dir).unwrap();
        let vendas = ler_vendas(Path::new("dados/vendas.csv"), Some(50)).expect("ok");
        let null = dir.join("null.avro");
        let deflate = dir.join("deflate.avro");
        let t_null = escrever_avro(&null, &vendas, Codec::Null).expect("ok");
        let t_def = escrever_avro(
            &deflate,
            &vendas,
            Codec::Deflate(DeflateSettings::default()),
        )
        .expect("ok");
        assert!(t_def <= t_null);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn avro_datum_round_trip() {
        let venda = Venda {
            id: 1,
            data: "2024-01-01".to_string(),
            categoria: "x".to_string(),
            produto: "y".to_string(),
            quantidade: 1,
            preco_unitario: 1.0,
            regiao: "Sul".to_string(),
            cliente_id: 1,
        };
        let schema_avro = schema().unwrap();
        let value = apache_avro::to_value(venda.clone()).unwrap();
        let buf = GenericDatumWriter::builder(&schema_avro)
            .build()
            .unwrap()
            .write_value_to_vec(value)
            .unwrap();
        let decoded_value = GenericDatumReader::builder(&schema_avro)
            .build()
            .unwrap()
            .read_value(&mut buf.as_slice())
            .unwrap();
        let decoded: Venda = from_value(&decoded_value).unwrap();
        assert_eq!(venda, decoded);
    }

    #[test]
    fn schema_e_valido() {
        assert!(schema().is_ok());
    }
}
