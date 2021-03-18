use crate::generate::*;
use crate::node::*;
use crate::parse::*;
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unary {
    Absolute,
    Negate,
    PreIncrement,
    PreDecrement,
    PostIncrement,
    PostDecrement,
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
        "number" => Syntax::Number(tree.values[0].parse().ok()?),
        "absolute" => unary(Unary::Absolute)?,
        "negate" => unary(Unary::Negate)?,
        "pre-increment" => unary(Unary::PreIncrement)?,
        "pre-decrement" => unary(Unary::PreDecrement)?,
        "post-increment" => unary(Unary::PostIncrement)?,
        "post-decrement" => unary(Unary::PostDecrement)?,
        "add" => binary(Binary::Add)?,
        "subtract" => binary(Binary::Subtract)?,
        "multiply" => binary(Binary::Multiply)?,
        "divide" => binary(Binary::Divide)?,
        _ => panic!("Invalid kind '{}'.", tree.kind),
    })
}

pub fn node() -> Node {
    let digit = || all!('0'..='9');
    fn wrap<N: ToNode>(node: N) -> Node {
        let space = repeat(.., any!(' ', '\n', '\r', '\t'));
        all!(space.clone(), node, space)
    }
    fn unary<N: ToNode>(operator: N) -> Node {
        prefix(100, wrap(all!(operator, &"expression")))
    }
    fn binary<N: ToNode>(operator: N, precedence: usize, bind: Bind) -> Node {
        postfix(precedence, bind, wrap(all!(wrap(operator), &"expression")))
    }
    all!(
        define(
            "expression",
            precede(
                any!(
                    &"group",
                    &"pre-increment",
                    &"pre-decrement",
                    &"absolute",
                    &"negate",
                    &"number"
                ),
                any!(
                    &"post-increment",
                    &"post-decrement",
                    &"add",
                    &"subtract",
                    &"multiply",
                    &"divide",
                )
            )
        ),
        define(
            "group",
            prefix(100, all!(wrap('('), &"expression", wrap(')')))
        ),
        syntax("number", prefix(100, wrap(all!(repeat(1.., digit()))))),
        syntax("absolute", unary('+')),
        syntax("negate", unary('-')),
        syntax("pre-increment", unary("++")),
        syntax("pre-decrement", unary("--")),
        syntax("post-increment", postfix(120, Bind::Left, "++")),
        syntax("post-decrement", postfix(120, Bind::Left, "--")),
        syntax("add", binary('+', 10, Bind::Left)),
        syntax("subtract", binary('-', 10, Bind::Left)),
        syntax("multiply", binary('*', 20, Bind::Left)),
        syntax("divide", binary('/', 20, Bind::Left)),
    )
}

pub fn parse(text: &str) -> Option<Syntax> {
    Parser::from(and(&"expression", node()))
        .parse(text)
        .and_then(|tree| convert(&tree))
}

pub fn generate() -> Option<String> {
    Generator::from(and(&"expression", node())).generate()
}
