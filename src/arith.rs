use crate::node::*;
use crate::parse::*;
use crate::*;

pub enum Unary {
    Negate,
    Decrement,
}

pub enum Binary {
    Add,
    Subtract,
    Multiply,
    Divide,
}

pub enum Syntax {
    Number(u64),
    Unary(Unary, Box<Syntax>),
    Binary(Binary, Box<Syntax>, Box<Syntax>),
}

pub fn convert(tree: &Tree) -> Option<Syntax> {
    let unary = |unary| -> Option<Syntax> {
        Some(Syntax::Unary(unary, convert(tree.children.get(0)?)?.into()))
    };
    let binary = |binary| -> Option<Syntax> {
        Some(Syntax::Binary(
            binary,
            convert(tree.children.get(0)?)?.into(),
            convert(tree.children.get(1)?)?.into(),
        ))
    };
    Some(match tree.kind.as_str() {
        "root" => convert(tree.children.first()?)?,
        "number" => Syntax::Number(tree.value.parse().ok()?),
        "negate" => unary(Unary::Negate)?,
        "decrement" => unary(Unary::Decrement)?,
        "add" => binary(Binary::Add)?,
        "subtract" => binary(Binary::Subtract)?,
        "multiply" => binary(Binary::Multiply)?,
        "divide" => binary(Binary::Divide)?,
        _ => panic!("Invalid kind '{}'.", tree.kind),
    })
}

pub fn node() -> Node {
    let digit = || all!('0'..='9');
    all!(
        &"expression",
        define!("expression", infix(&"prefix", &"postfix")),
        define!(
            "prefix",
            prefix(
                100,
                any!(
                    spawn!("negate", '-', &"expression"),
                    spawn!("group", '(', &"expression", ')'),
                    spawn!("number", repeat(1.., digit()))
                )
            )
        ),
        define!(
            "postfix",
            any!(
                spawn!("decrement", postfix(120, Bind::Left, "--")),
                spawn!(
                    "multiply",
                    postfix(20, Bind::Left, all!('*', &"expression"))
                ),
                spawn!("divide", postfix(20, Bind::Left, all!('/', &"expression"))),
                spawn!("add", postfix(10, Bind::Left, all!('+', &"expression"))),
                spawn!(
                    "subtract",
                    postfix(10, Bind::Left, all!('-', &"expression"))
                )
            )
        )
    )
}

pub fn parse(text: &str) -> Option<Syntax> {
    parse::parse(text, node()).and_then(|tree| convert(&tree))
}

pub fn generate() -> Option<String> {
    generate::generate(node())
}
