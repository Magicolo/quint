pub mod node;
pub mod parse;

use node::*;

pub fn json() -> Node {
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
                spawn!("boolean", any!("true", "false")),
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
