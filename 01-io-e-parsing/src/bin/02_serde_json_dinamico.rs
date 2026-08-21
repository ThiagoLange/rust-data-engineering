//! Módulo 01 — I/O e Parsing
//! README: seção "Conteúdo", item 2 — serde_json com `Value` para schemas dinâmicos
//!
//! No exemplo anterior (`01_serde_basico`) o schema é conhecido em tempo de
//! compilação: definimos um struct e o `serde` sabe exatamente que campos
//! esperar. Mas em engenharia de dados é comum receber JSON cujo formato
//! varia por registro — eventos de analytics de origens diferentes, webhooks
//! de terceiros, logs de aplicação heterogêneos. Para esses casos, `serde_json`
//! oferece o tipo `Value`: uma enum que representa *qualquer* JSON válido
//! (`Object`, `Array`, `String`, `Number`, `Bool`, `Null`).
//!
//! `Value` troca segurança de tipos em tempo de compilação por flexibilidade
//! em tempo de execução — é o equivalente, em Rust, de trabalhar com um
//! `dict`/`Map` genérico. A regra prática: use `Value` na borda do sistema
//! (ingestão de dados de schema variável) e converta para structs tipados
//! assim que possível, para o resto do código se beneficiar da checagem do
//! compilador.

use serde_json::Value;

/// Extrai um evento de analytics genérico e retorna um resumo legível,
/// tratando os diferentes formatos de campo "usuário" que cada origem pode
/// mandar (`user_id` como número, `userId` como string, ou ausente).
///
/// `Value` é navegado com `.get("campo")`, que retorna `Option<&Value>` —
/// não existe "campo não existe" travando o programa, existe `None` para
/// tratar explicitamente. Isso espelha o que aconteceria com dados reais de
/// produção: nem todo evento tem a mesma forma.
fn resumir_evento(evento: &Value) -> String {
    let tipo = evento
        .get("tipo")
        .and_then(Value::as_str)
        .unwrap_or("desconhecido");

    let usuario = extrair_usuario(evento);

    format!("evento={tipo} usuario={usuario}")
}

/// Tenta várias formas conhecidas de identificar o usuário dentro de um
/// evento, na ordem em que decidimos priorizá-las. Cada `origem` de dado
/// real costuma ter uma convenção diferente — isso é o preço de integrar
/// múltiplas fontes sem um schema único.
fn extrair_usuario(evento: &Value) -> String {
    if let Some(id) = evento.get("user_id").and_then(Value::as_u64) {
        return id.to_string();
    }
    if let Some(id) = evento.get("userId").and_then(Value::as_str) {
        return id.to_string();
    }
    "anonimo".to_string()
}

/// Percorre um `Value::Array` de eventos e soma quantos têm um campo
/// específico, não importa o tipo desse campo. Mostra como caminhar por uma
/// estrutura de JSON arbitrária usando pattern matching sobre a enum
/// `Value`, em vez de acessar campos que talvez nem existam.
fn contar_com_campo(eventos: &Value, campo: &str) -> usize {
    match eventos {
        Value::Array(itens) => itens
            .iter()
            .filter(|item| matches!(item, Value::Object(mapa) if mapa.contains_key(campo)))
            .count(),
        _ => 0,
    }
}

fn main() -> anyhow::Result<()> {
    let bruto = r#"[
        {"tipo": "clique", "user_id": 501, "elemento": "botao-comprar"},
        {"tipo": "pageview", "userId": "anon-88f2", "pagina": "/produtos"},
        {"tipo": "erro", "mensagem": "timeout ao carregar carrinho"},
        {"tipo": "clique", "user_id": 502, "elemento": "botao-cancelar", "metadata": {"origem": "mobile"}}
    ]"#;

    let eventos: Value = serde_json::from_str(bruto)?;

    let lista = eventos.as_array().expect("esperava um array de eventos");
    for evento in lista {
        println!("{}", resumir_evento(evento));
    }

    let com_user_id = contar_com_campo(&eventos, "user_id");
    println!("\nEventos com 'user_id' numérico: {com_user_id}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrai_usuario_de_campo_numerico() {
        let evento: Value = serde_json::from_str(r#"{"user_id": 42}"#).unwrap();
        assert_eq!(extrair_usuario(&evento), "42");
    }

    #[test]
    fn extrai_usuario_de_campo_string_alternativo() {
        let evento: Value = serde_json::from_str(r#"{"userId": "abc-123"}"#).unwrap();
        assert_eq!(extrair_usuario(&evento), "abc-123");
    }

    #[test]
    fn evento_sem_identificacao_de_usuario_usa_anonimo() {
        let evento: Value = serde_json::from_str(r#"{"tipo": "erro"}"#).unwrap();
        assert_eq!(extrair_usuario(&evento), "anonimo");
    }

    #[test]
    fn conta_apenas_objetos_com_o_campo_pedido() {
        let eventos: Value = serde_json::from_str(
            r#"[{"user_id": 1}, {"tipo": "erro"}, {"user_id": 2, "tipo": "clique"}]"#,
        )
        .unwrap();
        assert_eq!(contar_com_campo(&eventos, "user_id"), 2);
    }
}
