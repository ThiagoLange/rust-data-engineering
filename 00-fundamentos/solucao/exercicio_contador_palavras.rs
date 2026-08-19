// README: 00-fundamentos § "Exercício"
//
// Solução do exercício: contador de palavras paralelo com `rayon`, que
// processa múltiplos arquivos de texto simultaneamente e agrega os
// resultados com segurança — sem `unsafe`, sem race conditions.
//
// A diferença chave em relação ao exemplo `04_concorrencia_threads.rs`: lá
// usamos `Arc<Mutex<_>>` porque as threads precisavam ESCREVER num mesmo
// contador compartilhado. Aqui evitamos o Mutex por completo: cada thread
// do rayon processa um arquivo inteiro e devolve seu PRÓPRIO `HashMap`
// independente — ninguém compartilha estado mutável. Só depois, numa etapa
// separada de "reduce", esses mapas parciais são combinados. Esse padrão
// (map paralelo -> reduce) é geralmente mais simples de raciocinar do que
// um contador compartilhado, e o compilador garante a segurança dos dois
// jeitos.
//
// Uso:
//   cargo run --release --bin exercicio_contador_palavras -- dados/textos

use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Lê um arquivo e conta a frequência de cada palavra nele.
/// Normaliza pra minúsculas e descarta pontuação/símbolos de markdown
/// (`#`, `*`, `-`, etc.) — só letras e números contam como parte de uma
/// palavra.
fn contar_palavras_arquivo(caminho: &Path) -> Result<HashMap<String, usize>> {
    let conteudo = fs::read_to_string(caminho)
        .with_context(|| format!("falha ao ler arquivo {}", caminho.display()))?;

    let mut contagem = HashMap::new();
    for palavra_bruta in conteudo.split_whitespace() {
        let palavra: String = palavra_bruta
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        if !palavra.is_empty() {
            *contagem.entry(palavra).or_insert(0usize) += 1;
        }
    }
    Ok(contagem)
}

/// Combina dois mapas de contagem em um só, somando as frequências de
/// palavras que aparecem nos dois. Função pura — não muta nada fora dos
/// parâmetros que já são donos dos dados (`acc` é consumido e devolvido).
fn combinar_contagens(
    mut acc: HashMap<String, usize>,
    outro: HashMap<String, usize>,
) -> HashMap<String, usize> {
    for (palavra, quantidade) in outro {
        *acc.entry(palavra).or_insert(0) += quantidade;
    }
    acc
}

/// Processa todos os arquivos em paralelo (`par_iter`) e agrega os
/// resultados com `try_reduce`. Cada thread do rayon lê e conta "seu"
/// arquivo isoladamente — não existe estado compartilhado durante o
/// processamento, então não existe seção crítica pra proteger, e portanto
/// nenhuma race condition possível: cada `HashMap` intermediário pertence a
/// uma única thread até o momento em que é combinado.
fn contar_palavras_em_paralelo(caminhos: &[PathBuf]) -> Result<HashMap<String, usize>> {
    caminhos
        .par_iter()
        .map(|caminho| contar_palavras_arquivo(caminho))
        .try_reduce(HashMap::new, |acc, contagem| {
            Ok(combinar_contagens(acc, contagem))
        })
}

/// Lista os arquivos `.md` (ou `.txt`) dentro de um diretório, ordenados
/// por nome pra ter saída determinística.
fn listar_arquivos_de_texto(diretorio: &Path) -> Result<Vec<PathBuf>> {
    let mut arquivos: Vec<PathBuf> = fs::read_dir(diretorio)
        .with_context(|| format!("falha ao ler diretório {}", diretorio.display()))?
        .filter_map(|entrada| entrada.ok())
        .map(|entrada| entrada.path())
        .filter(|caminho| {
            matches!(
                caminho.extension().and_then(|ext| ext.to_str()),
                Some("md") | Some("txt")
            )
        })
        .collect();
    arquivos.sort();
    Ok(arquivos)
}

fn main() -> Result<()> {
    let diretorio = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "dados/textos".to_string());
    let diretorio = Path::new(&diretorio);

    let arquivos = listar_arquivos_de_texto(diretorio)?;
    println!("Processando {} arquivo(s) em paralelo...", arquivos.len());

    let contagem_total = contar_palavras_em_paralelo(&arquivos)?;

    let mut palavras_ordenadas: Vec<(&String, &usize)> = contagem_total.iter().collect();
    palavras_ordenadas.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));

    println!("\nTop 10 palavras mais frequentes:");
    for (palavra, quantidade) in palavras_ordenadas.iter().take(10) {
        println!("  {palavra}: {quantidade}");
    }
    println!("\nTotal de palavras distintas: {}", contagem_total.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn escrever_arquivo_temporario(nome: &str, conteudo: &str) -> PathBuf {
        let caminho =
            std::env::temp_dir().join(format!("{nome}-{:?}.md", std::thread::current().id()));
        let mut arquivo =
            fs::File::create(&caminho).expect("falha ao criar arquivo temporário de teste");
        arquivo
            .write_all(conteudo.as_bytes())
            .expect("falha ao escrever arquivo temporário de teste");
        caminho
    }

    #[test]
    fn contar_palavras_arquivo_normaliza_maiusculas_e_pontuacao() {
        let caminho =
            escrever_arquivo_temporario("teste-normalizacao", "# Rust é ótimo! Rust, rust rust.");
        let contagem = contar_palavras_arquivo(&caminho).expect("arquivo válido");
        assert_eq!(contagem.get("rust"), Some(&4));
        assert_eq!(contagem.get("é"), Some(&1));
        assert_eq!(contagem.get("ótimo"), Some(&1));
        let _ = fs::remove_file(caminho);
    }

    #[test]
    fn combinar_contagens_soma_frequencias_repetidas() {
        let mut a = HashMap::new();
        a.insert("rust".to_string(), 2);
        a.insert("dados".to_string(), 1);

        let mut b = HashMap::new();
        b.insert("rust".to_string(), 3);
        b.insert("tokio".to_string(), 1);

        let combinado = combinar_contagens(a, b);
        assert_eq!(combinado.get("rust"), Some(&5));
        assert_eq!(combinado.get("dados"), Some(&1));
        assert_eq!(combinado.get("tokio"), Some(&1));
    }

    #[test]
    fn contar_palavras_em_paralelo_agrega_varios_arquivos() {
        let caminho_a = escrever_arquivo_temporario("teste-paralelo-a", "rust rust dados");
        let caminho_b = escrever_arquivo_temporario("teste-paralelo-b", "dados dados tokio");

        let resultado = contar_palavras_em_paralelo(&[caminho_a.clone(), caminho_b.clone()])
            .expect("arquivos válidos");

        assert_eq!(resultado.get("rust"), Some(&2));
        assert_eq!(resultado.get("dados"), Some(&3));
        assert_eq!(resultado.get("tokio"), Some(&1));

        let _ = fs::remove_file(caminho_a);
        let _ = fs::remove_file(caminho_b);
    }

    #[test]
    fn listar_arquivos_de_texto_filtra_por_extensao_e_ordena() {
        let arquivos =
            listar_arquivos_de_texto(Path::new("dados/textos")).expect("diretório existe");
        assert!(arquivos.len() >= 3);
        assert!(arquivos.windows(2).all(|par| par[0] <= par[1]));
    }
}
