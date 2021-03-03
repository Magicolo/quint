extern crate quint;
use quint::node::*;
use quint::parse::*;

#[test]
fn boba() {
    let node = word("Boba");
    assert_eq!(true, parse("Boba", &node).is_some());
    assert_eq!(true, parse("Fett", &node).is_none());
}

#[test]
fn boba_and_fett() {
    let node = all(vec![word("Boba"), character(' '), word("Fett")]);
    assert_eq!(true, parse("Boba Fett", &node).is_some());
}

#[test]
fn boba_or_fett() {
    let node = any(vec![word("Boba"), word("Fett")]);
    assert_eq!(true, parse("Boba", &node).is_some());
    assert_eq!(true, parse("Fett", &node).is_some());
}

#[test]
fn many_boba() {
    let node = join(word("Boba"), option(character(' ')));
    assert_eq!(true, parse("Boba Boba", &node).is_some());
    assert_eq!(true, parse("BobaBoba", &node).is_some());
    assert_eq!(true, parse("Boba Boba Boba", &node).is_some());
    assert_eq!(true, parse("Boba BobaBoba Boba", &node).is_some());
}

#[test]
fn spawn_boba() {
    let node = spawn("Boba", word("Fett"));
    let tree = parse("Fett", &node).unwrap();
    assert_eq!("Boba", tree.children[0].kind);
    assert_eq!("Fett", tree.children[0].value);
}
