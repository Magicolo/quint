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
        "pre.number" => Syntax::Number(tree.values[0].parse().ok()?),
        "pre.absolute" => unary(Unary::Absolute)?,
        "pre.negate" => unary(Unary::Negate)?,
        "pre.increment" => unary(Unary::PreIncrement)?,
        "pre.decrement" => unary(Unary::PreDecrement)?,
        "post.increment" => unary(Unary::PostIncrement)?,
        "post.decrement" => unary(Unary::PostDecrement)?,
        "post.add" => binary(Binary::Add)?,
        "post.subtract" => binary(Binary::Subtract)?,
        "post.multiply" => binary(Binary::Multiply)?,
        "post.divide" => binary(Binary::Divide)?,
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
        prefix(100, wrap(all!(operator, &"")))
    }
    fn binary<N: ToNode>(operator: N, precedence: usize, bind: Bind) -> Node {
        postfix(precedence, bind, wrap(all!(wrap(operator), &"")))
    }
    all!(
        define(".expression", precede(&"pre", &"post")),
        define("pre.group", prefix(100, all!(wrap('('), &"", wrap(')')))),
        syntax(
            "pre.number",
            prefix(100, wrap(store(all!(repeat(1.., digit())))))
        ),
        syntax("pre.absolute", unary('+')),
        syntax("pre.negate", unary('-')),
        syntax("pre.increment", unary("++")),
        syntax("pre.decrement", unary("--")),
        syntax("post.increment", postfix(120, Bind::Left, "++")),
        syntax("post.decrement", postfix(120, Bind::Left, "--")),
        syntax("post.add", binary('+', 10, Bind::Left)),
        syntax("post.subtract", binary('-', 10, Bind::Left)),
        syntax("post.multiply", binary('*', 20, Bind::Left)),
        syntax("post.divide", binary('/', 20, Bind::Left)),
    )
}

pub fn parse(text: &str) -> Option<Syntax> {
    Parser::from(and(&"", node()))
        .parse(text)
        .first()
        .and_then(|tree| convert(&tree))
}

pub fn generate() -> Option<String> {
    Generator::from(and(&"", node())).generate()
}
