use std::collections::HashMap;
use std::ops::{Bound, RangeBounds};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Identifier {
    Unique(usize),
    Path(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Bind {
    None,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Set {
    Value(isize),
    Add(isize),
    Copy(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum If {
    Less,
    Equal,
    Greater,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Node {
    True,
    False,
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Define(Identifier, Box<Self>),
    Refer(Identifier),

    // Set(Identifier, Set),
    // If(Identifier, If, Identifier),
    // Push(Box<Self>),
    Precede(usize, Bind, Box<Self>),
    Spawn(Box<Self>),
    Symbol(char),
}

pub struct Context<T: Clone> {
    pub references: HashMap<usize, T>,
    pub identifiers: HashMap<String, usize>,
}

pub trait ToNode {
    fn node(self) -> Node;
}

impl ToNode for Node {
    fn node(self) -> Node {
        self
    }
}

impl ToNode for &&str {
    fn node(self) -> Node {
        refer(self)
    }
}

impl<T: Clone> Context<T> {
    pub fn new() -> Self {
        Self {
            references: HashMap::new(),
            identifiers: HashMap::new(),
        }
    }

    pub fn resolve(&mut self, node: Node) -> Node {
        node.descend(|node| match node {
            Node::And(left, right) if *left == Node::True => *right,
            Node::And(left, right) if *right == Node::True => *left,
            Node::And(left, _) if *left == Node::False => Node::False,
            Node::Or(left, right) if *left == Node::False => *right,
            Node::Or(left, right) if *right == Node::False => *left,
            Node::Or(left, _) if *left == Node::True => Node::True,
            _ => node,
        })
        .descend(|node| match node {
            Node::Define(identifier, node) => {
                Node::Define(Identifier::Unique(self.identify(&identifier)), node)
            }
            _ => node,
        })
    }

    pub fn identify(&mut self, identifier: &Identifier) -> usize {
        match identifier {
            Identifier::Unique(identifier) => *identifier,
            Identifier::Path(path) => {
                if let Some(identifier) = self.identifiers.get(path) {
                    *identifier
                } else {
                    let identifier = Node::unique();
                    self.identifiers.insert(path.clone(), identifier);
                    identifier
                }
            }
        }
    }

    pub fn refer(&mut self, identifier: &Identifier, reference: T) {
        let identifier = self.identify(identifier);
        self.references.insert(identifier, reference);
    }

    pub fn identifier(&self, identifier: &Identifier) -> Option<usize> {
        Some(match identifier {
            Identifier::Unique(identifier) => *identifier,
            Identifier::Path(path) => *self.identifiers.get(path)?,
        })
    }

    pub fn reference(&self, identifier: &Identifier) -> Option<T> {
        Some(self.references.get(&self.identifier(identifier)?)?.clone())
    }
}

impl Node {
    pub fn unique() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    pub fn descend(self, map: impl FnMut(Self) -> Self) -> Self {
        fn next(node: Node, map: &mut impl FnMut(Node) -> Node) -> Node {
            let node = match node {
                Node::And(left, right) => and(next(*left, map), next(*right, map)),
                Node::Or(left, right) => or(next(*left, map), next(*right, map)),
                Node::Define(identifier, node) => Node::Define(identifier, next(*node, map).into()),
                Node::Spawn(node) => Node::Spawn(next(*node, map).into()),
                Node::Precede(precedence, bind, node) => {
                    Node::Precede(precedence, bind, next(*node, map).into())
                }
                _ => node,
            };
            map(node)
        }
        let mut map = map;
        next(self, &mut map)
    }

    pub fn flatten(&self) -> Vec<&Node> {
        fn all<'a>(node: &'a Node, nodes: &mut Vec<&'a Node>) {
            match node {
                Node::And(left, right) => {
                    all(left, nodes);
                    all(right, nodes);
                }
                _ => nodes.push(node),
            }
        }

        fn any<'a>(node: &'a Node, nodes: &mut Vec<&'a Node>) {
            match node {
                Node::Or(left, right) => {
                    any(left, nodes);
                    any(right, nodes);
                }
                _ => nodes.push(node),
            }
        }

        let mut nodes = Vec::new();
        match self {
            Node::And(_, _) => all(self, &mut nodes),
            Node::Or(_, _) => any(self, &mut nodes),
            _ => nodes.push(self),
        }
        nodes
    }
}

pub fn option<N: ToNode>(node: N) -> Node {
    or(node, Node::True)
}

pub fn or<L: ToNode, R: ToNode>(left: L, right: R) -> Node {
    Node::Or(left.node().into(), right.node().into())
}

pub fn and<L: ToNode, R: ToNode>(left: L, right: R) -> Node {
    Node::And(left.node().into(), right.node().into())
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

pub fn repeat<R: RangeBounds<usize>, N: ToNode>(range: R, node: N) -> Node {
    let node = node.node();
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
            let refer = Node::Refer(identifier.clone());
            let node = option(and(node, refer.clone()));
            let define = Node::Define(identifier, node.into());
            and(define, refer)
        }
    };
    and(left, right)
}

pub fn refer(name: &str) -> Node {
    Node::Refer(Identifier::Path(name.into()))
}

pub fn define<N: ToNode>(path: &str, node: N) -> Node {
    Node::Define(Identifier::Path(path.into()), node.node().into())
}

pub fn join<S: ToNode, N: ToNode>(separator: S, node: N) -> Node {
    let node = node.node();
    option(and(node.clone(), repeat(.., and(separator, node))))
}

pub fn spawn<N: ToNode>(node: N) -> Node {
    Node::Spawn(node.node().into())
}

#[macro_export]
macro_rules! all {
    () => {{ Node::True }};
    ($node: expr) => {{ ToNode::node($node) }};
    ($node: expr, $($nodes: expr),+) => {{ and($node, all!($($nodes),+)) }};
    ($node: expr, $($nodes: expr),+,) => {{ and($node, all!($($nodes),+)) }};
}

#[macro_export]
macro_rules! any {
    () => {{ Node::False }};
    ($node: expr) => {{ ToNode::node($node) }};
    ($node: expr, $($nodes: expr),+) => {{ or($node, any!($($nodes),+)) }};
    ($node: expr, $($nodes: expr),+,) => {{ or($node, any!($($nodes),+)) }};
}
