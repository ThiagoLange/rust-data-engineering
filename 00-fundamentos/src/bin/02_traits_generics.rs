// README: 00-fundamentos § "Traits e generics"
//
// Trait = "interface" (se você vem de Java/TS) ou "protocol" (se vem de
// Python/Swift): um contrato que diz "todo tipo que implementar isso tem
// esses métodos". Generics = escrever código que funciona pra QUALQUER tipo
// que satisfaça um contrato, sem repetir a lógica pra cada tipo concreto.
//
// Em engenharia de dados isso é o que te permite escrever UMA função de
// pipeline (`aplicar`) que funciona pra normalizar números, maiúsculizar
// texto, ou qualquer outra transformação futura — sem copiar e colar o loop
// toda vez que aparece um tipo novo de dado.

/// O contrato: "qualquer coisa que sabe transformar um `T` em outro `T`".
/// `Transform<T>` é genérico sobre `T` — o mesmo trait serve pra transformar
/// `f64`, `String`, ou uma struct customizada, desde que cada `impl` diga
/// qual `T` ele trata.
trait Transform<T> {
    fn aplicar(&self, entrada: T) -> T;
}

/// Primeira implementação concreta: normalização min-max de um `f64`.
/// Guarda o mínimo e o máximo do dataset pra poder normalizar cada valor
/// pro intervalo [0.0, 1.0].
struct NormalizarMinMax {
    minimo: f64,
    maximo: f64,
}

impl Transform<f64> for NormalizarMinMax {
    fn aplicar(&self, entrada: f64) -> f64 {
        // Guarda contra divisão por zero quando todos os valores são iguais.
        if (self.maximo - self.minimo).abs() < f64::EPSILON {
            0.0
        } else {
            (entrada - self.minimo) / (self.maximo - self.minimo)
        }
    }
}

/// Segunda implementação: deixa uma `String` inteira em maiúsculas.
/// Mesma trait, tipo completamente diferente (`String` em vez de `f64`) —
/// é isso que "genérico sobre T" quer dizer na prática.
struct ParaMaiusculas;

impl Transform<String> for ParaMaiusculas {
    fn aplicar(&self, entrada: String) -> String {
        entrada.to_uppercase()
    }
}

/// Função genérica: recebe QUALQUER `Transform<T>` e aplica em cada elemento
/// de um `Vec<T>`. `Tr: Transform<T>` é um "trait bound" — diz ao compilador
/// "só aceito tipos que implementam esse contrato". O compilador gera código
/// especializado pra cada combinação usada (monomorphization) — ou seja,
/// zero custo de runtime comparado a escrever o loop na mão pra cada tipo.
fn aplicar_em_lote<T, Tr: Transform<T>>(dados: Vec<T>, transformacao: &Tr) -> Vec<T> {
    dados
        .into_iter()
        .map(|item| transformacao.aplicar(item))
        .collect()
}

/// Trait objects (`dyn Transform<f64>`): às vezes você não sabe em tempo de
/// compilação QUAL implementação vai usar — por exemplo, um pipeline
/// configurável por arquivo de config que escolhe a transformação em
/// runtime. Aí generics não servem (precisam saber o tipo concreto em
/// compile time); usa-se um "trait object" via `Box<dyn Transform<f64>>`,
/// que troca um pouco de performance (uma indireção, vtable) por
/// flexibilidade em runtime.
fn escolher_transformacao(usar_normalizacao: bool) -> Box<dyn Transform<f64>> {
    if usar_normalizacao {
        Box::new(NormalizarMinMax {
            minimo: 0.0,
            maximo: 100.0,
        })
    } else {
        // Transformação identidade — closures não implementam nosso trait
        // diretamente, então definimos uma struct minúscula só pra isso.
        struct Identidade;
        impl Transform<f64> for Identidade {
            fn aplicar(&self, entrada: f64) -> f64 {
                entrada
            }
        }
        Box::new(Identidade)
    }
}

fn main() {
    let notas = vec![10.0, 50.0, 100.0, 75.0];
    let normalizador = NormalizarMinMax {
        minimo: 0.0,
        maximo: 100.0,
    };
    let normalizadas = aplicar_em_lote(notas, &normalizador);
    println!("Notas normalizadas: {normalizadas:?}");

    let nomes = vec!["polars".to_string(), "datafusion".to_string()];
    let maiusculizador = ParaMaiusculas;
    let nomes_maiusculos = aplicar_em_lote(nomes, &maiusculizador);
    println!("Nomes em maiúsculas: {nomes_maiusculos:?}");

    // Aqui a decisão de QUAL transformação usar só existe em runtime —
    // é exatamente o caso de uso de `Box<dyn Transform<f64>>`.
    let transformacao_dinamica = escolher_transformacao(true);
    println!(
        "Transformação dinâmica aplicada a 42.0: {}",
        transformacao_dinamica.aplicar(42.0)
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizar_min_max_mapeia_para_zero_e_um() {
        let n = NormalizarMinMax {
            minimo: 0.0,
            maximo: 100.0,
        };
        assert_eq!(n.aplicar(0.0), 0.0);
        assert_eq!(n.aplicar(100.0), 1.0);
        assert_eq!(n.aplicar(50.0), 0.5);
    }

    #[test]
    fn normalizar_min_max_evita_divisao_por_zero() {
        let n = NormalizarMinMax {
            minimo: 5.0,
            maximo: 5.0,
        };
        assert_eq!(n.aplicar(5.0), 0.0);
    }

    #[test]
    fn aplicar_em_lote_funciona_para_qualquer_tipo_com_transform() {
        let maiusculizador = ParaMaiusculas;
        let resultado = aplicar_em_lote(vec!["rust".to_string()], &maiusculizador);
        assert_eq!(resultado, vec!["RUST".to_string()]);
    }

    #[test]
    fn trait_object_permite_escolha_em_runtime() {
        let t = escolher_transformacao(false);
        assert_eq!(t.aplicar(42.0), 42.0);
    }
}
