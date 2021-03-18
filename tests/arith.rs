extern crate quint;
use quint::arith::*;

fn test(text: &str, syntax: Syntax) {
    assert_eq!(syntax, parse(text).unwrap());
}

#[test]
fn add() {
    test(
        r#"1+2"#,
        Syntax::Binary(
            Binary::Add,
            Syntax::Number(1).into(),
            Syntax::Number(2).into(),
        ),
    );
    test(
        r#"1+2+3"#,
        Syntax::Binary(
            Binary::Add,
            Syntax::Number(1).into(),
            Syntax::Binary(
                Binary::Add,
                Syntax::Number(2).into(),
                Syntax::Number(3).into(),
            )
            .into(),
        ),
    );
    assert_eq!(true, parse(r#"1+"#).is_none());
    assert_eq!(true, parse(r#"+1"#).is_none());
}

#[test]
fn negate() {
    test(
        r#"-1"#,
        Syntax::Unary(Unary::Negate, Syntax::Number(1).into()),
    );
    test(
        r#"--1"#,
        Syntax::Unary(
            Unary::Negate,
            Syntax::Unary(Unary::Negate, Syntax::Number(1).into()).into(),
        ),
    );
    test(
        r#"---1"#,
        Syntax::Unary(
            Unary::Negate,
            Syntax::Unary(
                Unary::Negate,
                Syntax::Unary(Unary::Negate, Syntax::Number(1).into()).into(),
            )
            .into(),
        ),
    );
}

#[test]
fn generate_mixed() {
    for _ in 0..100 {
        let text = generate().unwrap();
        println!("{}", text);
        parse(&text).unwrap();
    }
}
