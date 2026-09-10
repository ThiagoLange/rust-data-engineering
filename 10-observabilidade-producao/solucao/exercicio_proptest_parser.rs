//! Módulo 10 — Observabilidade e Produção
//! README: seção "Exercício" — proptest no parser do Módulo 1
//!
//! Gera entradas aleatórias (incluindo malformadas) e valida que o parser
//! nunca entra em pânico, sempre retornando `Result` bem formado.

use anyhow::Result;

/// Parser simples de linha CSV `id,nome,valor` — espelha Módulo 1.
/// Nunca deve panicar; retorna Err para malformadas.
fn parse_linha(linha: &str) -> Result<(u64, String, f64)> {
    let partes: Vec<&str> = linha.split(',').collect();
    if partes.len() != 3 {
        anyhow::bail!("esperado 3 campos, obtido {}", partes.len());
    }
    let id: u64 = partes[0]
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("id inválido: {:?}", partes[0]))?;
    let nome = partes[1].trim().to_string();
    if nome.is_empty() {
        anyhow::bail!("nome vazio");
    }
    let valor: f64 = partes[2]
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("valor inválido: {:?}", partes[2]))?;
    if !valor.is_finite() {
        anyhow::bail!("valor não-finito");
    }
    Ok((id, nome, valor))
}

fn main() -> Result<()> {
    println!("=== Exercício proptest — Módulo 10 ===\n");
    println!("Rode `cargo test` para executar as propriedades.");
    println!("Exemplo manual:");
    for linha in ["1,arroz,10.5", "x,y", "1,,2.0", "1,a,NaN"] {
        match parse_linha(linha) {
            Ok((id, nome, valor)) => println!("  {linha:?} → Ok({id},{nome},{valor})"),
            Err(e) => println!("  {linha:?} → Err({e})"),
        }
    }
    println!("\n✓ Parser nunca entra em pânico (ver testes proptest)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::test_runner::TestRunner;

    proptest! {
        // Propriedade 1: parser nunca panica para qualquer string
        #[test]
        fn nunca_panica(s in "\\PC*") {
            let _ = parse_linha(&s);
        }

        // Propriedade 2: linha bem formada sempre parseia
        #[test]
        fn bem_formada_parseia(
            id in 0u64..1_000_000u64,
            nome in "[a-zA-Z]{1,20}",
            valor in -1e6f64..1e6f64
        ) {
            prop_assume!(valor.is_finite());
            let linha = format!("{id},{nome},{valor}");
            let res = parse_linha(&linha);
            prop_assert!(res.is_ok(), "linha {:?} deveria parsear: {:?}", linha, res);
            let (i, n, v) = res.unwrap();
            prop_assert_eq!(i, id);
            prop_assert_eq!(n, nome);
            prop_assert!((v - valor).abs() < 1e-9);
        }

        // Propriedade 3: número errado de campos sempre falha (sem pânico)
        #[test]
        fn campos_errados_falham(
            campos in prop::collection::vec("[a-z0-9]{0,10}", 0..5)
        ) {
            let linha = campos.join(",");
            let res = parse_linha(&linha);
            if campos.len() != 3 {
                prop_assert!(res.is_err());
            }
        }

        // Propriedade 4: id não-numérico sempre falha
        #[test]
        fn id_invalido_falha(s in "[a-zA-Z]+") {
            let linha = format!("{s},nome,1.0");
            prop_assert!(parse_linha(&linha).is_err());
        }
    }

    #[test]
    fn exemplos_manuais() {
        assert!(parse_linha("1,arroz,10.5").is_ok());
        assert!(parse_linha("x,y").is_err());
        assert!(parse_linha("1,,2.0").is_err());
        let _ = TestRunner::default();
    }
}
