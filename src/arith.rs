use crate::node::*;
use crate::parse::*;
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unary {
    Negate,
    Decrement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binary {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
    let binary =
        |symbol: char, precedence, bind| postfix(precedence, bind, all!(symbol, &"expression"));
    all!(
        define(
            "expression",
            precede(
                any!(&"negate", &"group", &"number"),
                any!(&"decrement", &"multiply", &"divide", &"add", &"subtract")
            )
        ),
        define("group", prefix(100, all!('(', &"expression", ')'))),
        define("negate", spawn(prefix(100, all!('-', &"expression")))),
        define("number", spawn(prefix(100, all!(repeat(1.., digit()))))),
        define("decrement", spawn(postfix(120, Bind::Left, "--"))),
        define("multiply", spawn(binary('*', 20, Bind::Left))),
        define("divide", spawn(binary('/', 20, Bind::Left))),
        define("add", spawn(binary('+', 10, Bind::Left))),
        define("subtract", spawn(binary('-', 10, Bind::Left))),
    )
}

pub fn parse(text: &str) -> Option<Syntax> {
    parse::parse(text, "expression", node()).and_then(|tree| convert(&tree))
}

pub fn generate() -> Option<String> {
    generate::generate(node(), "expression")
}
