extern crate quint;
use quint::arith;
use quint::json;
use quint::node::*;
use quint::parse::*;
use quint::*;

#[test]
fn boba() {
    let node = all!("Boba");
    assert_eq!(true, parse("Boba", &node).is_some());
    assert_eq!(true, parse("Fett", &node).is_none());
}

#[test]
fn boba_and_fett() {
    let node = all!("Boba", ' ', "Fett");
    assert_eq!(true, parse("Boba Fett", &node).is_some());
}

#[test]
fn boba_or_fett() {
    let node = any!("Boba", "Fett");
    assert_eq!(true, parse("Boba", &node).is_some());
    assert_eq!(true, parse("Fett", &node).is_some());
}

#[test]
fn repeat_boba() {
    let node = repeat(.., "Boba");
    assert_eq!(true, parse("", &node).is_some());
    assert_eq!(true, parse("Boba", &node).is_some());
    assert_eq!(true, parse("BobaBoba", &node).is_some());
    assert_eq!(true, parse("BobaBobaBoba", &node).is_some());
    assert_eq!(true, parse("BobaBobaBobaBoba", &node).is_some());
}

#[test]
fn repeat_low_boba() {
    let node = repeat(2.., "Boba");
    assert_eq!(true, parse("", &node).is_none());
    assert_eq!(true, parse("Boba", &node).is_none());
    assert_eq!(true, parse("BobaBoba", &node).is_some());
    assert_eq!(true, parse("BobaBobaBoba", &node).is_some());
    assert_eq!(true, parse("BobaBobaBobaBoba", &node).is_some());
}

#[test]
fn repeat_high_boba() {
    let node = repeat(..3, "Boba");
    assert_eq!(true, parse("", &node).is_some());
    assert_eq!(true, parse("Boba", &node).is_some());
    assert_eq!(true, parse("BobaBoba", &node).is_some());
    assert_eq!(true, parse("BobaBobaBoba", &node).is_none());
    assert_eq!(true, parse("BobaBobaBobaBoba", &node).is_none());
    let node = repeat(..=3, "Boba");
    assert_eq!(true, parse("", &node).is_some());
    assert_eq!(true, parse("Boba", &node).is_some());
    assert_eq!(true, parse("BobaBoba", &node).is_some());
    assert_eq!(true, parse("BobaBobaBoba", &node).is_some());
    assert_eq!(true, parse("BobaBobaBobaBoba", &node).is_none());
}

#[test]
fn repeat_range_boba() {
    let node = repeat(2..3, "Boba");
    assert_eq!(true, parse("", &node).is_none());
    assert_eq!(true, parse("Boba", &node).is_none());
    assert_eq!(true, parse("BobaBoba", &node).is_some());
    assert_eq!(true, parse("BobaBobaBoba", &node).is_none());
    assert_eq!(true, parse("BobaBobaBobaBoba", &node).is_none());
    let node = repeat(2..=3, "Boba");
    assert_eq!(true, parse("", &node).is_none());
    assert_eq!(true, parse("Boba", &node).is_none());
    assert_eq!(true, parse("BobaBoba", &node).is_some());
    assert_eq!(true, parse("BobaBobaBoba", &node).is_some());
    assert_eq!(true, parse("BobaBobaBobaBoba", &node).is_none());
}

#[test]
fn join_boba() {
    let node = join(option(symbol(' ')), "Boba");
    assert_eq!(true, parse("Boba Boba", &node).is_some());
    assert_eq!(true, parse("BobaBoba", &node).is_some());
    assert_eq!(true, parse("Boba Boba Boba", &node).is_some());
    assert_eq!(true, parse("Boba BobaBoba Boba", &node).is_some());
}

#[test]
fn spawn_boba() {
    let node = spawn("Boba", "Fett");
    let tree = parse("Fett", &node).unwrap();
    assert_eq!("Boba", tree.children[0].kind);
    assert_eq!("Fett", tree.children[0].value);
}

#[test]
fn refer_boba_fett() {
    let node = all(vec![
        define("Boba", "Boba"),
        refer("Boba"),
        symbol(' '),
        refer("Fett"),
        define("Fett", "Fett"),
    ]);
    assert_eq!(true, parse("Boba Fett", &node).is_some());
}

#[test]
fn json_number() {
    let node = json::node();
    let tree = parse(r#"-1.2E3"#, &node).unwrap();
    assert_eq!("number", tree.children[0].kind);
    let tree = parse(r#"-0.1e2"#, &node).unwrap();
    assert_eq!("number", tree.children[0].kind);
}

// #[test]
// fn arith_add() {
//     let node = arith();
//     let tree = parse(r#"1+2"#, &node).unwrap();
//     assert_eq!("number", tree.children[0].kind);
//     assert_eq!("add", tree.children[1].kind);
//     let tree = parse(r#"1+2+3"#, &node).unwrap();
//     assert_eq!("number", tree.children[0].kind);
//     assert_eq!("add", tree.children[1].kind);
//     assert_eq!(true, parse(r#"1+"#, &node).is_none());
//     assert_eq!(true, parse(r#"+1"#, &node).is_none());
// }

// #[test]
// fn arith_negate() {
//     let node = arith();
//     let tree = parse(r#"-1"#, &node).unwrap();
//     assert_eq!("negate", tree.children[0].kind);
//     let tree = parse(r#"--1"#, &node).unwrap();
//     assert_eq!("negate", tree.children[0].kind);
//     let tree = parse(r#"---1"#, &node).unwrap();
//     assert_eq!("negate", tree.children[0].kind);
// }

#[test]
fn arith_mixed() {
    let node = arith::node();
    // let tree = parse(r#"1+2*3-4/5"#, &node).unwrap();
    // assert_eq!("number", tree.children[0].kind);
    // assert_eq!("add", tree.children[1].kind);
    // let tree = parse(r#"1+-2"#, &node).unwrap();
    // assert_eq!("number", tree.children[0].kind);
    // assert_eq!("add", tree.children[1].kind);
    let tree = parse(r#"-1--2"#, &node).unwrap();
    assert_eq!("negate", tree.children[0].kind);
}
