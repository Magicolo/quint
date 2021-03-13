extern crate quint;
use quint::json::*;
use quint::node::*;
use quint::*;

fn test(json: &str, syntax: Syntax) {
    assert_eq!(parse(json).unwrap(), syntax);
}

#[test]
fn aaaa() {
    // let mut context = Context::<()>::new();
    // let a = all!(
    //     &"value",
    //     define("value", any!(&"boolean", &"identifier", &"null", &"array")),
    //     syntax("identifier", repeat(2.., any!('t', 'r', 'u', 'e'))),
    //     syntax("boolean", any!("true", "false")),
    //     syntax("array", all!('[', join(',', &"value"), ']')),
    //     syntax("null", "null")
    // );
    // let a = all!(
    //     any!(repeat(1.., &"karl"), &"palss"),
    //     all!(define("karl", "aaaccc")),
    //     all!(define("palss", any!("aaabccc", "aadccc")))
    // );
    // let a = any!("aPPPbcg", "aPPbdg", "aPPaaabbbg", "aPPebdaaag");
    // context.resolve(a);

    /*
    'Store' behavior may be implicitly achievable by consuming the text since the last 'Syntax'
    - requires a 'consume' index in 'State'
    - will consume by doing: 'state.text[state.consume..state.index]; state.consume = state.index;'

    "jango boba karl jango fett karl" => [c, b, e, d, a] => { a: { b: c, d: e } }

    {
        'f': ("ett" & [e] & [a]),
        'j': ("ango" &
        {
            'b': ("oba" & [c] & "karl" & [b] & [a]),
            'f': ("ett" & [e] & "karl" & [d] & [a])
        }),
        'b': ("oba" & [c] & [a])
    }
    "boba" => [c, a] => { a: c }
    "fett" => [e, a] => { a: e }
    "jangobobakarl" => [c, b, a] => { a: { b: c } }
    "jangofettkarl" => [e, d, a] => { a: { d: e } }

    &a => { depth: 0 }
    &b & &d => { depth: 0 }
    &b { depth: 1 }
    */
    // let tree = parse::parse(
    //     "jangobobakarlbobajangofettkarlfett", // { a: { b: { c }, c, { d: e }, e } }
    //     all!(
    //         &"a",
    //         syntax("a", repeat(1.., any!(&"b", &"c", &"d", &"e"))),
    //         syntax("b", all!("jango", &"c", "karl")),
    //         syntax("c", "boba"),
    //         syntax("d", all!("jango", &"e", "karl")),
    //         syntax("e", "fett"),
    //     ),
    // );
    let tree = parse::parse(
        // "bobafett", // { a: { b: { c }, c, { d: e }, e } }
        // "jangobobabobabobakarlbobajangofettkarlfett", // { a: { b: { c }, c, { d: e }, e } }
        "{[{[[boba]]}]}", // { b: { c: { b: c } } }
        all!(
            &"b",
            // syntax("a", repeat(.., any!(&"b", &"c", &"d", &"e"))),
            syntax("b", all!("{", &"c", "}")),
            syntax("c", all!("[", option(any!(store("boba"), &"b", &"c")), "]")),
            // syntax("d", all!("jango", repeat(1.., &"e"), "karl")),
            // syntax("e", "fett"),
        ),
    );
    println!("{:?}", tree);
}

#[test]
fn number() {
    test(r#"-1.2E3"#, Syntax::Number(-1.2e3));
    test(r#"-0.1e2"#, Syntax::Number(-0.1e2));
}

#[test]
fn null() {
    test(r#"null"#, Syntax::Null);
}

#[test]
fn boolean_true() {
    test(r#"true"#, Syntax::Boolean(true));
}

#[test]
fn boolean_false() {
    test(r#"false"#, Syntax::Boolean(false));
}

#[test]
fn number_array() {
    test(
        r#"[0,1,2]"#,
        Syntax::Array(vec![
            Syntax::Number(0.),
            Syntax::Number(1.),
            Syntax::Number(2.),
        ]),
    );
}

#[test]
fn nested_array() {
    test(
        r#"[0,[1,[2]]]"#,
        Syntax::Array(vec![
            Syntax::Number(0.),
            Syntax::Array(vec![
                Syntax::Number(1.),
                Syntax::Array(vec![Syntax::Number(2.)]),
            ]),
        ]),
    );
}

#[test]
fn generate_mixed() {
    for _ in 0..100 {
        let text = generate().unwrap();
        parse(&text).unwrap();
    }
}
