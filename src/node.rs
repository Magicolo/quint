use std::ops::{Bound, RangeBounds};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Identifier {
    Unique(usize),
    Name(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Node {
    True,
    False,
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Definition(Identifier, Box<Self>),
    Reference(Identifier),

    Spawn(String, Box<Self>),
    Symbol(char),
}

pub trait AsNode {
    fn as_node(self) -> Node;
}

impl AsNode for Node {
    fn as_node(self) -> Node {
        self
    }
}

impl AsNode for &&str {
    fn as_node(self) -> Node {
        refer(self)
    }
}

impl Node {
    pub fn unique() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    pub fn descend<F: FnMut(Self) -> Self>(self, map: F) -> Self {
        fn next<F: FnMut(Node) -> Node>(node: Node, map: &mut F) -> Node {
            let node = match node {
                Node::And(left, right) => and(next(*left, map), next(*right, map)),
                Node::Or(left, right) => or(next(*left, map), next(*right, map)),
                Node::Definition(identifier, node) => {
                    Node::Definition(identifier, Box::new(next(*node, map)))
                }
                Node::Spawn(kind, node) => Node::Spawn(kind, Box::new(next(*node, map))),
                _ => node,
            };
            map(node)
        }
        let mut map = map;
        next(self, &mut map)
    }
}

pub fn option<N: AsNode>(node: N) -> Node {
    or(node, Node::True)
}

pub fn or<Left: AsNode, Right: AsNode>(left: Left, right: Right) -> Node {
    Node::Or(Box::new(left.as_node()), Box::new(right.as_node()))
}

pub fn and<Left: AsNode, Right: AsNode>(left: Left, right: Right) -> Node {
    Node::And(Box::new(left.as_node()), Box::new(right.as_node()))
}

pub fn any(nodes: Vec<Node>) -> Node {
    let mut nodes = nodes;
    nodes
        .drain(..)
        .rev()
        .fold(Node::False, |sum, node| or(node, sum))
}

pub fn all(nodes: Vec<Node>) -> Node {
    let mut nodes = nodes;
    nodes
        .drain(..)
        .rev()
        .fold(Node::True, |sum, node| and(node, sum))
}

pub fn chain(nodes: Vec<Node>) -> Node {
    let mut nodes = nodes;
    nodes
        .drain(..)
        .rev()
        .fold(Node::True, |sum, node| option(and(node, sum)))
}

pub fn repeat<R: RangeBounds<usize>, N: AsNode>(range: R, node: N) -> Node {
    let node = node.as_node();
    let bounds = (range.start_bound(), range.end_bound());
    let low = match bounds.0 {
        Bound::Included(index) => *index,
        Bound::Excluded(index) => index + 1,
        Bound::Unbounded => 0,
    };
    let high = match bounds.1 {
        Bound::Included(index) => Some(*index),
        Bound::Excluded(index) => Some(index - 1),
        Bound::Unbounded => None,
    };
    let left = all(vec![node.clone(); low]);
    let right = match high {
        Some(high) if high > low => chain(vec![node.clone(); high - low]),
        Some(_) => Node::True,
        None => {
            let identifier = Identifier::Unique(Node::unique());
            let node = option(and(node, Node::Reference(identifier.clone())));
            and(Node::Definition(identifier, Box::new(node.clone())), node)
        }
    };
    and(left, right)
}

pub fn refer(name: &str) -> Node {
    Node::Reference(Identifier::Name(name.into()))
}

pub fn define<N: AsNode>(name: &str, node: N) -> Node {
    Node::Definition(Identifier::Name(name.into()), Box::new(node.as_node()))
}

pub fn join<S: AsNode, N: AsNode>(separator: S, node: N) -> Node {
    repeat(.., and(node, option(separator)))
}

pub fn spawn<N: AsNode>(kind: &str, node: N) -> Node {
    and(
        define(kind, Node::Spawn(kind.into(), Box::new(node.as_node()))),
        refer(kind),
    )
}

#[macro_export]
macro_rules! all {
    () => {{ Node::True }};
    ($node: expr) => {{ AsNode::as_node($node) }};
    ($node: expr, $($nodes: expr),+) => {{ and($node, all!($($nodes),+)) }};
}

#[macro_export]
macro_rules! any {
    () => {{ Node::False }};
    ($node: expr) => {{ AsNode::as_node($node.as_node()) }};
    ($node: expr, $($nodes: expr),+) => {{ or($node, any!($($nodes),+)) }};
}

#[macro_export]
macro_rules! spawn {
    ($kind: expr, $($nodes: expr),+) => {{ spawn($kind, all!($($nodes),+)) }};
}

#[macro_export]
macro_rules! define {
    ($name: expr, $($nodes: expr),+) => {{ define($name, all!($($nodes),+)) }};
}

#[macro_export]
macro_rules! option {
    ($($nodes: expr),+) => {{ option(all!($($nodes),+)) }};
}
