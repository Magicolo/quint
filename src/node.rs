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
pub enum Node {
    True,
    False,
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Definition(Identifier, Box<Self>),
    Reference(Identifier),

    Precedence(usize, Bind, Box<Self>),
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
    pub fn resolve(node: Node) -> (Node, Context<T>) {
        let mut context = Context {
            references: HashMap::new(),
            identifiers: HashMap::new(),
        };
        let node = node
            .descend(|node| match node {
                Node::And(left, right) if *left == Node::True => *right,
                Node::And(left, right) if *right == Node::True => *left,
                Node::And(left, _) if *left == Node::False => Node::False,
                Node::Or(left, right) if *left == Node::False => *right,
                Node::Or(left, right) if *right == Node::False => *left,
                Node::Or(left, _) if *left == Node::True => Node::True,
                _ => node,
            })
            .descend(|node| match node {
                Node::Definition(identifier, node) => {
                    Node::Definition(Identifier::Unique(context.identifier(&identifier)), node)
                }
                _ => node,
            });
        (node, context)
    }

    pub fn identifier(&mut self, identifier: &Identifier) -> usize {
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

    pub fn reference(&self, identifier: &Identifier) -> Option<T> {
        Some(match identifier {
            Identifier::Unique(identifier) => self.references.get(identifier)?.clone(),
            Identifier::Path(path) => self
                .identifiers
                .get(path)
                .and_then(|identifier| self.references.get(identifier))?
                .clone(),
        })
    }

    pub fn add(&mut self, identifier: &Identifier, value: T) {
        let identifier = self.identifier(identifier);
        self.references.insert(identifier, value);
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
                    Node::Definition(identifier, next(*node, map).into())
                }
                Node::Spawn(node) => Node::Spawn(next(*node, map).into()),
                Node::Precedence(precedence, bind, node) => {
                    Node::Precedence(precedence, bind, next(*node, map).into())
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
                    all(left, nodes);
                    all(right, nodes);
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
            let node = option(and(node, Node::Reference(identifier.clone())));
            and(Node::Definition(identifier, node.clone().into()), node)
        }
    };
    and(left, right)
}

pub fn refer(name: &str) -> Node {
    Node::Reference(Identifier::Path(name.into()))
}

pub fn define<N: ToNode>(path: &str, node: N) -> Node {
    Node::Definition(Identifier::Path(path.into()), node.node().into())
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
}

#[macro_export]
macro_rules! any {
    () => {{ Node::False }};
    ($node: expr) => {{ ToNode::node($node) }};
    ($node: expr, $($nodes: expr),+) => {{ or($node, any!($($nodes),+)) }};
}
