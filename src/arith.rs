use crate::node::*;
use crate::parse::*;
use crate::*;

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
