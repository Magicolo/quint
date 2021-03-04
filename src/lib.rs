pub mod node;
pub mod parse;

use node::*;
use parse::*;

pub fn json() -> Node {
    /*
    string: C.all(
        P.character('"', false),
        C.loop(C.any(
            C.all(P.character("\\", false), C.any(
                P.any(["\\", '"', "/", "b", "f", "n", "r", "t"], false),
                C.all(P.character("u", false), C.repeat([4, 4], P.any([["0", "9"], ["a", "f"], ["A", "F"]], false)))
            )),
            P.none(['"', "\\", "\u0000", "\u0001"], false)
        )),
        P.character('"')
    ),
    number: C.all(
        C.optional(P.character("-", false)),
        C.any(P.character("0", false), C.all(P.range("1", "9", false), C.loop(P.range("0", "9", false)))),
        C.optional(C.all(P.character(".", false), C.repeat(1, P.range("0", "9", false)))),
        C.optional(C.all(
            P.any(["e", "E"], false),
            C.optional(C.any(P.character("+", false), P.character("-", false))),
            C.repeat(1, P.range("0", "9", false))
        ))
    ),
    */
    all(vec![
        define(
            "value",
            any(vec![
                refer("null"),
                refer("boolean"),
                refer("number"),
                refer("string"),
                refer("array"),
                refer("object"),
            ]),
        ),
        define(
            "pair",
            all(vec![refer("string"), word(":"), refer("value")]),
        ),
        define(
            "escape",
            and(
                symbol('\\'),
                any(vec![
                    symbol('\\'),
                    symbol('"'),
                    symbol('/'),
                    symbol('b'),
                    symbol('f'),
                    symbol('n'),
                    symbol('r'),
                    symbol('t'),
                ]),
            ),
        ),
        // define("hex", all()),
        spawn("null", word("null")),
        spawn("boolean", or(word("true"), word("false"))),
        // spawn("number"),
        spawn("string", all(vec![symbol('"'), symbol('"')])),
        spawn(
            "array",
            all(vec![word("["), join(refer("value"), word(",")), word("]")]),
        ),
        spawn(
            "object",
            all(vec![word("{"), join(refer("pair"), word(",")), word("}")]),
        ),
    ])
}
