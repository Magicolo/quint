extern crate quint;
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
    let node = json();
    let tree = parse(r#"-1.2E3"#, &node).unwrap();
    assert_eq!("number", tree.children[0].kind);
    let tree = parse(r#"-0.1e2"#, &node).unwrap();
    assert_eq!("number", tree.children[0].kind);
}
