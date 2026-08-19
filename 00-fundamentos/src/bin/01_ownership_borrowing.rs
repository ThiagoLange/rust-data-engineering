// README: 00-fundamentos § "Ownership e borrowing na prática"
//
// Este exemplo é o ponto de partida do módulo. Se você vem de Python/Java/JS,
// a ideia mais estranha do Rust é: toda vez que você "passa" um valor pra uma
// função, ou você está *emprestando* (borrow) ele, ou está *transferindo a
// posse* (move) dele. Não existe garbage collector decidindo depois — o
// compilador decide, em tempo de compilação, quem é dono de quê.
//
// Por que isso importa em engenharia de dados: pipelines processam lotes
// (batches) de registros que passam por várias etapas de transformação.
// Em linguagens com GC, é fácil ter duas partes do código segurando
// referência pro "mesmo" batch e uma delas mutar por baixo dos pés da outra
// (data race, ou um "já foi liberado" tipo use-after-free). Rust proíbe isso
// em tempo de compilação: ou você tem UMA referência mutável, ou VÁRIAS
// referências imutáveis — nunca as duas ao mesmo tempo.

/// Representa um registro de vendas bem simples — o tipo de coisa que
/// chegaria de um CSV, uma fila Kafka, etc.
#[derive(Debug, Clone)]
struct RegistroVenda {
    produto: String,
    valor: f64,
}

/// MOVE (transferência de posse): esta função recebe `Vec<RegistroVenda>`
/// por valor. Isso significa que quem chamar `total_vendas` PERDE a posse
/// do vetor — não dá mais pra usar a variável original depois da chamada,
/// a menos que a função devolva algo.
///
/// Por que isso é bom: o compilador garante que só existe UM dono do vetor
/// em cada instante. Não tem como duas threads "possuírem" o mesmo Vec e
/// uma liberar a memória enquanto a outra ainda lê (double-free).
fn total_vendas(registros: Vec<RegistroVenda>) -> f64 {
    // `registros` é dono do Vec aqui dentro. Quando a função termina, o Vec
    // é destruído (drop) automaticamente — sem precisar de `free()` manual
    // e sem GC rodando em background.
    registros.iter().map(|r| r.valor).sum()
}

/// BORROW imutável (`&`): esta função recebe uma *referência* ao vetor, não
/// o vetor em si. Quem chama continua dono e pode usar a variável depois.
/// Múltiplas partes do código podem ter `&` ao mesmo tempo — é seguro
/// porque nenhuma delas pode modificar os dados.
fn produto_mais_caro(registros: &[RegistroVenda]) -> Option<&RegistroVenda> {
    registros.iter().max_by(|a, b| {
        a.valor
            .partial_cmp(&b.valor)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

/// BORROW mutável (`&mut`): aplica um desconto in-place, sem copiar o vetor
/// inteiro nem transferir posse. O compilador garante que, enquanto essa
/// referência mutável existir, NENHUMA outra referência (mutável ou não)
/// pode acessar o mesmo vetor — é isso que elimina data races em tempo de
/// compilação, mesmo em código que ainda nem rodou.
fn aplicar_desconto(registros: &mut [RegistroVenda], percentual: f64) {
    for registro in registros.iter_mut() {
        registro.valor *= 1.0 - percentual;
    }
}

fn main() {
    let vendas = vec![
        RegistroVenda {
            produto: "teclado".into(),
            valor: 250.0,
        },
        RegistroVenda {
            produto: "monitor".into(),
            valor: 900.0,
        },
        RegistroVenda {
            produto: "mouse".into(),
            valor: 80.0,
        },
    ];

    // Emprestamos `&vendas` aqui — `vendas` continua válida depois.
    if let Some(mais_caro) = produto_mais_caro(&vendas) {
        println!(
            "Produto mais caro: {} (R$ {:.2})",
            mais_caro.produto, mais_caro.valor
        );
    }

    // Precisamos de uma cópia mutável pra aplicar desconto — `vendas` original
    // fica intacta porque usamos `.clone()` explicitamente. Rust nunca copia
    // por acidente: cópia é sempre uma decisão visível no código.
    let mut vendas_com_desconto = vendas.clone();
    aplicar_desconto(&mut vendas_com_desconto, 0.10);
    println!("Após 10% de desconto: {vendas_com_desconto:?}");

    // Aqui `vendas` é MOVIDA pra dentro de `total_vendas` — depois desta
    // linha, a variável `vendas` não existe mais neste escopo. Se você
    // descomentar o `println!("{vendas:?}")` logo abaixo, o compilador
    // recusa compilar com "value borrowed here after move" — é exatamente
    // esse tipo de erro que, em produção, vira bug de double-free em
    // linguagens sem esse dono único garantido pelo compilador.
    let total = total_vendas(vendas);
    println!("Total vendido (sem desconto): R$ {total:.2}");
    // println!("{vendas:?}"); // <- não compila: `vendas` foi movida acima
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amostra() -> Vec<RegistroVenda> {
        vec![
            RegistroVenda {
                produto: "a".into(),
                valor: 10.0,
            },
            RegistroVenda {
                produto: "b".into(),
                valor: 30.0,
            },
            RegistroVenda {
                produto: "c".into(),
                valor: 20.0,
            },
        ]
    }

    #[test]
    fn total_vendas_soma_todos_os_valores() {
        assert_eq!(total_vendas(amostra()), 60.0);
    }

    #[test]
    fn produto_mais_caro_encontra_o_maior_valor() {
        let registros = amostra();
        let mais_caro = produto_mais_caro(&registros).expect("lista não está vazia");
        assert_eq!(mais_caro.produto, "b");
        // `registros` ainda é válida aqui — prova de que `&` não consumiu o vetor.
        assert_eq!(registros.len(), 3);
    }

    #[test]
    fn aplicar_desconto_reduz_valores_in_place() {
        let mut registros = amostra();
        aplicar_desconto(&mut registros, 0.5);
        assert_eq!(registros[0].valor, 5.0);
        assert_eq!(registros[1].valor, 15.0);
        assert_eq!(registros[2].valor, 10.0);
    }
}
