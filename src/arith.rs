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
        "number" => Syntax::Number(tree.value.parse().ok()?),
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
        define(
            "number",
            spawn(prefix(100, wrap(all!(repeat(1.., digit())))))
        ),
        define("absolute", spawn(unary('+'))),
        define("negate", spawn(unary('-'))),
        define("pre-increment", spawn(unary("++"))),
        define("pre-decrement", spawn(unary("--"))),
        define("post-increment", spawn(postfix(120, Bind::Left, "++"))),
        define("post-decrement", spawn(postfix(120, Bind::Left, "--"))),
        define("add", spawn(binary('+', 10, Bind::Left))),
        define("subtract", spawn(binary('-', 10, Bind::Left))),
        define("multiply", spawn(binary('*', 20, Bind::Left))),
        define("divide", spawn(binary('/', 20, Bind::Left))),
    )
}

pub fn parse(text: &str) -> Option<Syntax> {
    parse::parse(text, "expression", node()).and_then(|tree| convert(&tree))
}

pub fn generate() -> Option<String> {
    generate::generate(node(), "expression")
}
