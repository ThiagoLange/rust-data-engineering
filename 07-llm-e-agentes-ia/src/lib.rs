//! Módulo 07 — LLM e Agentes: utilitários compartilhados.

use anyhow::Result;

/// Chunk de documento para RAG.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: usize,
    pub doc: String,
    pub text: String,
}

/// Chunking simples: janela 500 chars com overlap 50.
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = text.chars().collect();
    while start < chars.len() {
        let end = (start + chunk_size).min(chars.len());
        let chunk: String = chars[start..end].iter().collect();
        if !chunk.trim().is_empty() {
            chunks.push(chunk);
        }
        if end == chars.len() {
            break;
        }
        start = end.saturating_sub(overlap);
    }
    chunks
}

/// Embedding sintético determinístico (fallback quando sem API).
/// 32 dimensões, baseado em hash do texto — útil para testes e demo offline.
/// Na prática, seria `async-openai` ou `reqwest` para Anthropic.
pub fn fake_embed(text: &str) -> Vec<f32> {
    let mut v = vec![0f32; 32];
    for (i, c) in text.bytes().enumerate() {
        let idx = i % 32;
        v[idx] += c as f32 / 255.0;
    }
    // Normaliza para L2 unit (para DistDot/cosine)
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    for x in &mut v {
        *x /= norm;
    }
    v
}

/// Gera embeddings via API se `OPENAI_API_KEY` estiver setada, senão fake.
/// Versão async para demonstrar `async-openai`.
pub async fn embed(text: &str) -> Result<Vec<f32>> {
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            // Tenta API — se falhar, cai para fake
            if let Ok(v) = embed_via_openai(text, &key).await {
                return Ok(v);
            }
        }
    }
    Ok(fake_embed(text))
}

async fn embed_via_openai(text: &str, api_key: &str) -> Result<Vec<f32>> {
    use async_openai::{config::OpenAIConfig, types::CreateEmbeddingRequestArgs, Client};
    let config = OpenAIConfig::new().with_api_key(api_key);
    let client = Client::with_config(config);
    let req = CreateEmbeddingRequestArgs::default()
        .model("text-embedding-3-small")
        .input(text)
        .build()?;
    let resp = client.embeddings().create(req).await?;
    let emb = resp.data[0].embedding.clone();
    // Normaliza
    let norm = emb.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
    Ok(emb.into_iter().map(|x| x / norm).collect())
}

/// Chama LLM para gerar resposta a partir de prompt com contexto RAG.
/// Se `OPENAI_API_KEY` não estiver setada, retorna resposta simulada.
pub async fn call_llm(prompt: &str) -> Result<String> {
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        if !key.is_empty() {
            if let Ok(r) = call_openai(prompt, &key).await {
                return Ok(r);
            }
        }
    }
    // Fallback: verifica Anthropic via reqwest
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        if !key.is_empty() {
            if let Ok(r) = call_anthropic(prompt, &key).await {
                return Ok(r);
            }
        }
    }
    // Simulado — útil para demo offline e testes
    Ok(format!(
        "[LLM simulado — defina OPENAI_API_KEY ou ANTHROPIC_API_KEY para chamadas reais]\nPrompt:\n{prompt}\n\nResposta: Com base no contexto recuperado, esta é uma resposta sintética demonstrando o fluxo RAG."
    ))
}

async fn call_openai(prompt: &str, api_key: &str) -> Result<String> {
    use async_openai::{config::OpenAIConfig, types::CreateChatCompletionRequestArgs, Client};
    let config = OpenAIConfig::new().with_api_key(api_key);
    let client = Client::with_config(config);
    let req = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o-mini")
        .max_tokens(512u32)
        .messages([async_openai::types::ChatCompletionRequestMessage::User(
            async_openai::types::ChatCompletionRequestUserMessageArgs::default()
                .content(prompt)
                .build()?,
        )])
        .build()?;
    let resp = client.chat().create(req).await?;
    let content = resp.choices[0].message.content.clone().unwrap_or_default();
    Ok(content)
}

async fn call_anthropic(prompt: &str, api_key: &str) -> Result<String> {
    // Chamada direta via reqwest — útil quando não há SDK oficial atualizado
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "claude-3-haiku-20240307",
        "max_tokens": 512,
        "messages": [{"role": "user", "content": prompt}]
    });
    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;
    let json: serde_json::Value = resp.json().await?;
    let text = json["content"][0]["text"]
        .as_str()
        .unwrap_or("sem resposta")
        .to_string();
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunking_preserva_texto() {
        let text = "a".repeat(600);
        let chunks = chunk_text(&text, 500, 50);
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].len(), 500);
    }

    #[test]
    fn fake_embed_normalizado() {
        let v = fake_embed("hello world");
        assert_eq!(v.len(), 32);
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn fake_embed_deterministico() {
        let a = fake_embed("teste");
        let b = fake_embed("teste");
        assert_eq!(a, b);
        let c = fake_embed("outro");
        assert_ne!(a, c);
    }
}
