extern crate quint;
use quint::json::*;
use quint::node::*;
use quint::parse::*;
use quint::*;

fn test(json: &str, syntax: Syntax) {
    assert_eq!(parse(json).unwrap(), syntax);
}

#[test]
fn aaaa() {
    let tree = Parser::from(all!(
        &"a",
        syntax("a", repeat(.., any!(&"b", &"c", &"d", &"e"))),
        syntax("b.0", store(all!("{", &"c", "}"))),
        syntax("b.1", store(all!("<", &"c", ">"))),
        syntax("c", all!("[", any!(store("boba"), &"b", &"c"), "]")),
        syntax("d", all!("jango", repeat(1.., &"e"), "karl")),
        syntax("e", all!(store("fe"), option(store("tt")), store('a'..'z'))),
    ))
    .parse("fettafep{[[boba]]}jangofettifettukarl");
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
