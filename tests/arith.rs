extern crate quint;
use quint::arith::*;

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

// #[test]
// fn arith_mixed() {
//     let node = arith::node();
//     // let tree = parse(r#"1+2*3-4/5"#, &node).unwrap();
//     // assert_eq!("number", tree.children[0].kind);
//     // assert_eq!("add", tree.children[1].kind);
//     // let tree = parse(r#"1+-2"#, &node).unwrap();
//     // assert_eq!("number", tree.children[0].kind);
//     // assert_eq!("add", tree.children[1].kind);
//     let tree = parse(r#"-1--2"#, &node).unwrap();
//     assert_eq!("negate", tree.children[0].kind);
// }

#[test]
fn generate_mixed() {
    for _ in 0..100 {
        let text = generate().unwrap();
        println!("{}", text);
        parse(&text).unwrap();
    }
}
