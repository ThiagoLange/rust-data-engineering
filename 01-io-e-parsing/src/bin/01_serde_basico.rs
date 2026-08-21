//! Módulo 01 — I/O e Parsing
//! README: seção "Conteúdo", item 1 — serde
//!
//! `serde` não é um parser em si: é um framework de *serialização* e
//! *deserialização*. Ele define os traits `Serialize` e `Deserialize`, que
//! descrevem "como converter este tipo Rust em/de uma representação genérica
//! de dados" (um mapa de campos, uma lista, um número, etc). Quem sabe
//! transformar essa representação genérica em JSON, YAML, bincode, etc. são
//! outros crates (`serde_json`, `serde_yaml`, `bincode`...).
//!
//! Na prática, quase sempre usamos as derive macros `#[derive(Serialize,
//! Deserialize)]`, que geram esse código de conversão automaticamente a
//! partir da definição do struct — sem precisar escrever nada manual.

use serde::{Deserialize, Serialize};

/// Struct de exemplo representando um registro de funcionário, como viria
/// de uma API de RH ou de um arquivo de configuração.
///
/// `#[serde(rename = "...")]` é útil quando o nome do campo no Rust (que
/// segue a convenção `snake_case`) precisa mapear para um nome diferente na
/// origem dos dados (JSON com `camelCase`, ou nomes com caracteres que não
/// são identificadores Rust válidos, como "e-mail").
///
/// `#[serde(default)]` faz o campo assumir o valor padrão do tipo (`false`
/// para `bool`, `0` para números, `Vec::new()` para vetores...) quando ele
/// não aparece nos dados de entrada, em vez de falhar o parsing. Isso é
/// comum em engenharia de dados: fontes de dados evoluem e nem todo
/// registro antigo tem todos os campos novos.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Funcionario {
    nome: String,
    idade: u32,
    #[serde(rename = "e-mail")]
    email: String,
    #[serde(default)]
    ativo: bool,
}

/// Serializa um `Funcionario` para uma string JSON.
///
/// `serde_json::to_string` percorre o struct usando o trait `Serialize`
/// (implementado pela derive macro) e produz o JSON. Se o struct tivesse
/// algum campo que não sabe se serializar (o que não acontece aqui, já que
/// todos os tipos usados implementam `Serialize`), isso seria um erro de
/// compilação, não de runtime — outra vantagem de usar tipos fortes.
fn serializar(funcionario: &Funcionario) -> anyhow::Result<String> {
    let json = serde_json::to_string_pretty(funcionario)?;
    Ok(json)
}

/// Deserializa uma string JSON para `Funcionario`.
///
/// Se o JSON não bater com o schema esperado (campo obrigatório faltando,
/// tipo errado), `serde_json::from_str` retorna `Err` com uma mensagem que
/// já aponta linha e coluna do problema — muito mais útil do que um parser
/// manual de JSON escrito à mão.
fn deserializar(json: &str) -> anyhow::Result<Funcionario> {
    let funcionario = serde_json::from_str(json)?;
    Ok(funcionario)
}

fn main() -> anyhow::Result<()> {
    let funcionario = Funcionario {
        nome: "Ana Souza".to_string(),
        idade: 34,
        email: "ana.souza@empresa.com".to_string(),
        ativo: true,
    };

    let json = serializar(&funcionario)?;
    println!("Funcionário serializado:\n{json}\n");

    // Note o "e-mail" no JSON (por causa do rename) e a ausência de
    // "ativo" no exemplo abaixo — o `#[serde(default)]` cobre essa falta.
    let json_incompleto = r#"{
        "nome": "Bruno Lima",
        "idade": 28,
        "e-mail": "bruno.lima@empresa.com"
    }"#;

    let recuperado = deserializar(json_incompleto)?;
    println!("Funcionário deserializado (sem 'ativo' no JSON):\n{recuperado:#?}");
    assert!(!recuperado.ativo, "default de bool deve ser false");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_preserva_dados() {
        let original = Funcionario {
            nome: "Carla".to_string(),
            idade: 41,
            email: "carla@empresa.com".to_string(),
            ativo: true,
        };

        let json = serializar(&original).unwrap();
        let recuperado = deserializar(&json).unwrap();

        assert_eq!(original, recuperado);
    }

    #[test]
    fn rename_mapeia_email_para_e_mail() {
        let funcionario = Funcionario {
            nome: "Davi".to_string(),
            idade: 22,
            email: "davi@empresa.com".to_string(),
            ativo: false,
        };

        let json = serializar(&funcionario).unwrap();
        assert!(json.contains("\"e-mail\""));
        assert!(!json.contains("\"email\""));
    }

    #[test]
    fn campo_ausente_usa_default() {
        let json = r#"{"nome": "Eva", "idade": 30, "e-mail": "eva@empresa.com"}"#;
        let funcionario = deserializar(json).unwrap();
        assert!(!funcionario.ativo);
    }

    #[test]
    fn campo_obrigatorio_ausente_falha() {
        // "idade" é obrigatório (não tem #[serde(default)]) — sem ele,
        // o parsing deve falhar com um erro claro, não com um valor 0 mudo.
        let json = r#"{"nome": "Fabio", "e-mail": "fabio@empresa.com"}"#;
        let resultado = deserializar(json);
        assert!(resultado.is_err());
    }
}
