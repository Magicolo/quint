use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Identifier {
    Unique(usize),
    Name(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Identity,
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Identifier(Identifier, Box<Self>),
    Reference(Identifier),

    Spawn(String, Box<Self>),
    Character(char),
}

pub fn unique() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub fn option(node: Node) -> Node {
    or(node, Node::Identity)
}

pub fn or(left: Node, right: Node) -> Node {
    Node::Or(Box::new(left), Box::new(right))
}

pub fn and(left: Node, right: Node) -> Node {
    Node::And(Box::new(left), Box::new(right))
}

pub fn any(mut nodes: Vec<Node>) -> Node {
    nodes.drain(..).fold(Node::Identity, or)
}

pub fn all(mut nodes: Vec<Node>) -> Node {
    nodes.drain(..).fold(Node::Identity, and)
}

pub fn many(node: Node) -> Node {
    let identifier = Identifier::Unique(unique());
    Node::Identifier(
        identifier.clone(),
        Box::new(option(and(node, Node::Reference(identifier)))),
    )
}

pub fn refer(name: &str) -> Node {
    Node::Reference(Identifier::Name(name.into()))
}

pub fn identify(name: &str, node: Node) -> Node {
    Node::Identifier(Identifier::Name(name.into()), Box::new(node))
}

pub fn join(node: Node, separator: Node) -> Node {
    option(and(node.clone(), many(and(separator, node))))
}

pub fn spawn(kind: &str, node: Node) -> Node {
    identify(kind, Node::Spawn(kind.into(), Box::new(node)))
}
