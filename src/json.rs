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

fn convert(tree: &Tree) -> Option<Syntax> {
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
    all!(
        &"value",
        define!(
            "value",
            any!(
                spawn!("null", "null"),
                spawn!("true", "true"),
                spawn!("false", "false"),
                spawn!("string", '"', repeat(.., letter()), '"'),
                spawn!("array", '[', join(',', &"value"), ']'),
                spawn!("object", '{', join(',', pair()), '}'),
                spawn!(
                    "number",
                    option!('-'),
                    any!('0', all!('1'..='9', repeat(.., digit()))),
                    option!('.', repeat(1.., digit())),
                    option!(any!('e', 'E'), option(any!('+', '-')), repeat(1.., digit()))
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
