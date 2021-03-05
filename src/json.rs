use crate::node::*;
use crate::parse::*;
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Syntax {
    Null,
    Number(f64),
    Boolean(bool),
    String(String),
    Array(Vec<Syntax>),
    Object(Vec<(Syntax, Syntax)>),
}

pub fn convert(tree: &Tree) -> Option<Syntax> {
    Some(match tree.kind.as_str() {
        "root" => convert(tree.children.first()?)?,
        "null" => Syntax::Null,
        "number" => Syntax::Number(tree.value.parse().ok()?),
        "true" => Syntax::Boolean(true),
        "false" => Syntax::Boolean(false),
        "string" => Syntax::String(tree.value.into()),
        "array" => {
            let mut items = Vec::new();
            for child in tree.children.iter() {
                items.push(convert(child)?);
            }
            Syntax::Array(items)
        }
        "object" => {
            let mut pairs = Vec::new();
            let mut children = tree.children.iter();
            while let (Some(key), Some(value)) = (children.next(), children.next()) {
                pairs.push((convert(key)?, convert(value)?));
            }
            Syntax::Object(pairs)
        }
        _ => panic!("Invalid kind '{}'.", tree.kind),
    })
}

pub fn node() -> Node {
    let digit = || all!('0'..='9');
    let pair = || all!(&"string", ':', &"value");
    let hex = || all!('u', repeat(4..4, any!(digit(), 'a'..='f', 'A'..='F')));
    let escape = || all!('\\', any!('\\', '/', '"', 'b', 'f', 'n', 'r', 't', hex()));
    let letter = || any!(escape(), 'a'..='z', 'A'..'Z');
    let integer = || all!(option('-'), any!('0', all!('1'..='9', repeat(.., digit()))));
    let fraction = || all!('.', repeat(1.., digit()));
    let exponent = || all!(any!('e', 'E'), option(any!('+', '-')), repeat(1.., digit()));
    let number = || all!(integer(), option(fraction()), option(exponent()));
    all!(
        define(
            "value",
            any!(&"null", &"true", &"false", &"string", &"array", &"object", &"number")
        ),
        define("null", spawn("null")),
        define("true", spawn("true")),
        define("false", spawn("false")),
        define("string", all!('"', spawn(repeat(.., letter())), '"')),
        define("array", all!('[', spawn(join(',', &"value")), ']')),
        define("object", all!('{', spawn(join(',', pair())), '}')),
        define("number", spawn(number()))
    )
}

pub fn parse(text: &str) -> Option<Syntax> {
    parse::parse(text, "value", node()).and_then(|tree| convert(&tree))
}

pub fn generate() -> Option<String> {
    generate::generate(node(), "value")
}
