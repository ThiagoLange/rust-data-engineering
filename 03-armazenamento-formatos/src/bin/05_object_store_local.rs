//! Módulo 03 — Armazenamento e Formatos
//! README: seção "Conteúdo", item 5 — object_store local
//!
//! O crate `object_store` oferece uma API unificada para ler e escrever
//! objetos, seja em disco local, S3, GCS ou Azure. Este exemplo usa o backend
//! local (`LocalFileSystem`) para simular um bucket sem depender de nuvem.

use anyhow::{Context, Result};
use futures::TryStreamExt;
use object_store::local::LocalFileSystem;
use object_store::{ObjectStore, PutPayload};
use std::path::{absolute, Path};
use std::sync::Arc;

fn criar_store(caminho: &Path) -> Result<Arc<dyn ObjectStore>> {
    let abs = absolute(caminho).with_context(|| format!("resolvendo {}", caminho.display()))?;
    Ok(Arc::new(LocalFileSystem::new_with_prefix(abs)?))
}

#[tokio::main]
async fn main() -> Result<()> {
    let bucket = Path::new("dados/saida/bucket_local");
    std::fs::create_dir_all(bucket).context("criando bucket local")?;

    let store = criar_store(bucket)?;

    let conteudo = b"ola, este e um objeto no object_store local";
    let payload = PutPayload::from_static(conteudo);
    store
        .put(&object_store::path::Path::from("mensagem.txt"), payload)
        .await
        .context("escrevendo objeto")?;
    println!("Objeto escrito em: {}/mensagem.txt", bucket.display());

    let metas = store
        .list(None)
        .try_collect::<Vec<_>>()
        .await
        .context("listando objetos")?;
    println!("Objetos no bucket:");
    for meta in &metas {
        println!("  {} ({} bytes)", meta.location, meta.size);
    }

    let bytes = store
        .get(&object_store::path::Path::from("mensagem.txt"))
        .await
        .context("lendo objeto")?
        .bytes()
        .await
        .context("lendo bytes")?;
    println!("Conteúdo lido: {}", String::from_utf8_lossy(&bytes));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[tokio::test]
    async fn put_list_get_funcionam() {
        let dir = env::temp_dir().join("object_store_test");
        std::fs::create_dir_all(&dir).unwrap();
        let store = criar_store(&dir).expect("ok");

        let payload = PutPayload::from_static(b"teste");
        store
            .put(&object_store::path::Path::from("x.txt"), payload)
            .await
            .expect("ok");

        let metas = store.list(None).try_collect::<Vec<_>>().await.expect("ok");
        assert_eq!(metas.len(), 1);

        let bytes = store
            .get(&object_store::path::Path::from("x.txt"))
            .await
            .expect("ok")
            .bytes()
            .await
            .expect("ok");
        assert_eq!(bytes.as_ref(), b"teste");

        std::fs::remove_dir_all(dir).unwrap();
    }
}
