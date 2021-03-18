extern crate quint;
use quint::node::*;
use quint::parse::*;
use quint::*;

fn test(text: &str, node: Node, success: bool) {
    assert_eq!(success, Parser::from(node).parse(text).is_some());
}

#[test]
fn boba() {
    let node = all!("Boba");
    test("Boba", node.clone(), true);
    test("Fett", node.clone(), false);
}

#[test]
fn boba_and_fett() {
    test("Boba Fett", all!("Boba", ' ', "Fett"), true);
}

#[test]
fn boba_or_fett() {
    let node = any!("Boba", "Fett");
    test("Boba", node.clone(), true);
    test("Fett", node.clone(), true);
}

#[test]
fn repeat_boba() {
    let node = repeat(.., "Boba");
    test("", node.clone(), true);
    test("Boba", node.clone(), true);
    test("BobaBoba", node.clone(), true);
    test("BobaBobaBoba", node.clone(), true);
    test("BobaBobaBobaBoba", node.clone(), true);
}

#[test]
fn repeat_low_boba() {
    let node = repeat(2.., "Boba");
    test("", node.clone(), false);
    test("Boba", node.clone(), false);
    test("BobaBoba", node.clone(), true);
    test("BobaBobaBoba", node.clone(), true);
    test("BobaBobaBobaBoba", node.clone(), true);
}

#[test]
fn repeat_high_boba() {
    let node = repeat(..3, "Boba");
    test("", node.clone(), true);
    test("Boba", node.clone(), true);
    test("BobaBoba", node.clone(), true);
    test("BobaBobaBoba", node.clone(), false);
    test("BobaBobaBobaBoba", node.clone(), false);
    let node = repeat(..=3, "Boba");
    test("", node.clone(), true);
    test("Boba", node.clone(), true);
    test("BobaBoba", node.clone(), true);
    test("BobaBobaBoba", node.clone(), true);
    test("BobaBobaBobaBoba", node.clone(), false);
}

#[test]
fn repeat_range_boba() {
    let node = repeat(2..3, "Boba");
    test("", node.clone(), false);
    test("Boba", node.clone(), false);
    test("BobaBoba", node.clone(), true);
    test("BobaBobaBoba", node.clone(), false);
    test("BobaBobaBobaBoba", node.clone(), false);
    let node = repeat(2..=3, "Boba");
    test("", node.clone(), false);
    test("Boba", node.clone(), false);
    test("BobaBoba", node.clone(), true);
    test("BobaBobaBoba", node.clone(), true);
    test("BobaBobaBobaBoba", node.clone(), false);
}

#[test]
fn join_boba() {
    let node = join(option(text(' ')), "Boba");
    test("Boba Boba", node.clone(), true);
    test("BobaBoba", node.clone(), true);
    test("Boba Boba Boba", node.clone(), true);
    test("Boba BobaBoba Boba", node.clone(), true);
}

#[test]
fn spawn_boba() {
    let tree = Parser::from(and(&"Boba", define("Boba", store("Fett"))))
        .parse("Fett")
        .unwrap();
    assert_eq!("Boba", tree.kind);
    assert_eq!("Fett", tree.values[0]);
}

#[test]
fn refer_boba_fett() {
    let node = all(vec![
        define("Boba", "Boba"),
        refer("Boba"),
        text(' '),
        refer("Fett"),
        define("Fett", "Fett"),
    ]);
    test("Boba Fett", node.clone(), true);
}
