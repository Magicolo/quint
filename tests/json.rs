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
    let tree = parse::parse(
        "[pp][pp]",
        all!(
            any!(repeat(.., &"branch")),
            syntax("leaf", any!("p", "o", "u")),
            syntax("branch", all!("[", repeat(1.., &"leaf"), "]"))
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
