use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::fmt::{Display, Error, Formatter};
use std::mem;
use std::ops::{Bound, RangeBounds};
use std::str;
use std::sync::atomic::{AtomicUsize, Ordering};
use Identifier::*;
use Node::*;

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

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Node {
    True,
    False,
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Define(Identifier, Box<Self>),
    Refer(Identifier),
    Precede(usize, Bind, Box<Self>),

    Symbol(char),
    Spawn(String),
    Depth(usize, Box<Self>),
    Store(usize, Box<Self>),
    Switch(Vec<(char, Node)>),
    Text(String),
    /*
    State nodes:
        Set(Identifier, Set),
        If(Identifier, If, Identifier),
        Push(),
        Pop(),

    - if-else:
        Or(And(If("left", compare, "right"), if), else)
    - indent:
        And(
            Set("indent", Value(0)),
            Loop(And(
                Symbol('\t'),
                Set("indent", Add(1))
            )),
            If("indent", >, "$.indent"),
            Set("$.index", Copy("index")),
        )
    - dedent:
        And(
            Set("indent", Value(0)),
            Loop(And(
                Symbol('\t'),
                Set("indent", Add(1))
            )),
            If("indent", <, "$.indent"),
            Set("$.index", Copy("index")),
        )
    - precedence:
        And(
            Set("precedence", Value(precedence * 2)),
            If("precedence", >, "$.precedence"),
            Push(),
            Set("$.precedence", Value(Bind::Left => precedence * 2, Bind::Right => precedence * 2 - 1)),
            node,
            Pop(),
        )
    */
}

pub struct Context<T: Clone> {
    pub definitions: HashMap<usize, Node>,
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

impl ToNode for Box<Node> {
    fn node(self) -> Node {
        *self
    }
}

impl ToNode for &&str {
    fn node(self) -> Node {
        refer(self)
    }
}

impl ToNode for bool {
    fn node(self) -> Node {
        if self {
            True
        } else {
            False
        }
    }
}

impl<T: Clone> Context<T> {
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            references: HashMap::new(),
            identifiers: HashMap::new(),
        }
    }

    pub fn resolve(&mut self, node: Node) -> Node {
        // TODO: Apply precedence properly.
        // TODO: Find a way to optimize through 'Store/Precedence' nodes.
        // TODO: Find a way to reproduce 'push/pop' behavior of 'state.path' that 'Refer' nodes do.

        fn identify<T: Clone>(node: Node, context: &mut Context<T>) -> Node {
            match node {
                Define(identifier, node) => {
                    context.define(&identifier, *node);
                    True
                }
                Refer(identifier) => Refer(Unique(context.identify(&identifier))),
                node => node,
            }
        }

        fn precede(node: Node, precedence: usize) -> Node {
            let (node, precedence) = match node {
                Precede(current, bind, node) => match bind {
                    Bind::Left if current <= precedence => (False, 0),
                    Bind::Right if current < precedence => (False, 0),
                    _ => (*node, current),
                },
                node => (node, precedence),
            };
            node.map(|node| precede(node, precedence))
        }

        /// (a & b) | (a & c) => a & (b | c)
        fn factor(node: Node) -> Node {
            match node {
                And(left, right) => match (*left, *right) {
                    (And(left, middle), right) => {
                        factor(And(left, factor(And(middle, right.into())).into()))
                    }
                    (left, right) => and(left, right),
                },
                Or(left, right) => match (*left, *right) {
                    (Or(left, middle), right) => {
                        factor(Or(left, factor(Or(middle, right.into())).into()))
                    }
                    (And(left1, right1), And(mut left2, right2)) if left1 == left2 => {
                        *left2 = factor(Or(right1, right2));
                        factor(And(left1, left2))
                    }
                    (left, right) => or(left, right),
                },
                node => node,
            }
        }

        /// 'a' => { 'a': True }, "ab" => { 'a': True } & { 'b': True }
        fn pre(node: Node) -> Node {
            match node {
                Text(text) => all(text
                    .chars()
                    .map(|symbol| Switch(vec![(symbol, True)]))
                    .collect()),
                Symbol(symbol) => Switch(vec![(symbol, True)]),
                node => node,
            }
        }

        fn process(node: Node) -> Node {
            match node {
                And(left, right) => match (*left, *right) {
                    (True, right) => right,
                    (left, True) => left,
                    (False, _) => False,
                    (_, False) => False,
                    (Switch(mut left), right) => {
                        for case in left.iter_mut() {
                            let value = mem::replace(&mut case.1, True);
                            case.1 = process(and(value, right.clone()));
                        }
                        Switch(left)
                    }
                    (left, right) => and(left, right),
                },
                Or(left, right) => match (*left, *right) {
                    (left, right) if left == right => left,
                    (False, right) => right,
                    (left, False) => left,
                    (True, right) => process(or(right, True)),
                    (Switch(mut left), Switch(mut right)) => {
                        left.append(&mut right);
                        process(Switch(left))
                    }
                    (left, right) => or(left, right),
                },
                Switch(cases) if cases.len() == 0 => True,
                Switch(mut cases) => {
                    let mut map = HashMap::new();
                    for case in cases.drain(..) {
                        match map.get_mut(&case.0) {
                            Some(value) => {
                                *value = process(or(mem::replace(value, True), case.1));
                            }
                            None => {
                                map.insert(case.0, case.1);
                            }
                        };
                    }
                    for (key, value) in map {
                        cases.push((key, value));
                    }
                    Switch(cases)
                }
                Text(text) if text.len() == 0 => True,
                node => node,
            }
        }

        /// { 'a': b } => 'a' & b, 'a' & 'b' => "ab"
        fn post(node: Node) -> Node {
            match node {
                And(left, right) => match (*left, *right) {
                    (And(left, middle), right) => post(And(left, And(middle, right.into()).into())),
                    (Text(mut left), Text(right)) => {
                        left.push_str(right.as_str());
                        Text(left)
                    }
                    (Text(mut left), And(middle, right)) => match *middle {
                        Text(middle) => {
                            left.push_str(middle.as_str());
                            post(And(Text(left).into(), right))
                        }
                        middle => and(Text(left), and(middle, right)),
                    },
                    (left, right) => and(left, right),
                },
                Switch(mut cases) if cases.len() == 1 => {
                    let case = cases.pop().unwrap();
                    post(process(and(Text(case.0.into()), case.1)))
                }
                Symbol(symbol) => Text(symbol.into()),
                node => node,
            }
        }

        fn expand<T: Clone>(
            node: Node,
            context: &mut Context<T>,
            set: &mut HashSet<usize>,
        ) -> Node {
            match node {
                Refer(Unique(identifier)) => {
                    if set.insert(identifier) {
                        let node = context.definitions[&identifier].clone();
                        let node = optimize(node, context, set);
                        context.definitions.insert(identifier, node.clone());
                        node
                    } else {
                        context.definitions[&identifier].clone()
                    }
                }
                node => node,
            }
        }

        fn dig(node: Node, depth: usize) -> Node {
            match node {
                Depth(inner, node) => dig(*node, inner + depth),
                Spawn(kind) if depth > 0 => Depth(depth, Spawn(kind).into()),
                Refer(identifier) if depth > 0 => Depth(depth, Refer(identifier).into()),
                node => node.map(|node| dig(node, depth)),
            }
        }

        fn store(node: Node, offset: usize) -> Node {
            println!("STORE: {} -> {}", offset, node);
            match node {
                And(left, right) => match store(*left, offset) {
                    Symbol(symbol) => {
                        and(Symbol(symbol), store(*right, offset + symbol.len_utf8()))
                    }
                    Text(text) => {
                        let offset = offset + text.len();
                        and(Text(text), store(*right, offset))
                    }
                    left => And(left.into(), right),
                },
                Store(inner, node) => store(*node, if offset > inner { offset } else { inner }),
                Spawn(kind) if offset > 0 => Store(offset, Spawn(kind).into()),
                Refer(identifier) if offset > 0 => Store(offset, Refer(identifier).into()),
                node => node.map(|node| store(node, offset)),
            }
        }

        fn optimize<T: Clone>(
            node: Node,
            context: &mut Context<T>,
            set: &mut HashSet<usize>,
        ) -> Node {
            let node = node
                .descend(|node| expand(node, context, set))
                .descend(factor);
            store(dig(node, 0), 0)
                .descend(pre)
                .descend(process)
                .descend(post)
        }

        println!("Original");
        println!("{}", node);
        let node = node.descend(|node| identify(node, self));
        let node = optimize(node, self, &mut HashSet::new());
        println!("Optimize");
        println!("{}", node);
        for pair in self.definitions.iter() {
            println!("{}: {}", pair.0, pair.1);
        }
        node
    }

    pub fn define(&mut self, identifier: &Identifier, node: Node) {
        let identifier = self.identify(identifier);
        self.definitions.insert(identifier, node);
    }

    pub fn identify(&mut self, identifier: &Identifier) -> usize {
        match identifier {
            Unique(identifier) => *identifier,
            Path(path) => {
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
            Unique(identifier) => *identifier,
            Path(path) => *self.identifiers.get(path)?,
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

    pub fn map(self, map: impl FnMut(Self) -> Self) -> Self {
        let mut map = map;
        match self {
            And(mut left, mut right) => {
                *left = map(*left);
                *right = map(*right);
                And(left, right)
            }
            Or(mut left, mut right) => {
                *left = map(*left);
                *right = map(*right);
                Or(left, right)
            }
            Define(identifier, mut node) => {
                *node = map(*node);
                Define(identifier, node)
            }
            Depth(depth, mut node) => {
                *node = map(*node);
                Depth(depth, node)
            }
            Store(offset, mut node) => {
                *node = map(*node);
                Store(offset, node)
            }
            Precede(precedence, bind, mut node) => {
                *node = map(*node);
                Precede(precedence, bind, node)
            }
            Switch(mut cases) => {
                for case in cases.iter_mut() {
                    let value = mem::replace(&mut case.1, True);
                    case.1 = map(value);
                }
                Switch(cases)
            }
            node => node,
        }
    }

    pub fn descend(self, map: impl FnMut(Self) -> Self) -> Self {
        fn next(node: Node, map: &mut impl FnMut(Node) -> Node) -> Node {
            let node = node.map(|node| next(node, map));
            map(node)
        }

        let mut map = map;
        next(self, &mut map)
    }

    pub fn flatten(&self) -> Vec<&Node> {
        fn all<'a>(node: &'a Node, nodes: &mut Vec<&'a Node>) {
            match node {
                And(left, right) => {
                    all(left, nodes);
                    all(right, nodes);
                }
                _ => nodes.push(node),
            }
        }

        fn any<'a>(node: &'a Node, nodes: &mut Vec<&'a Node>) {
            match node {
                Or(left, right) => {
                    any(left, nodes);
                    any(right, nodes);
                }
                _ => nodes.push(node),
            }
        }

        let mut nodes = Vec::new();
        match self {
            And(_, _) => all(self, &mut nodes),
            Or(_, _) => any(self, &mut nodes),
            _ => nodes.push(self),
        }
        nodes
    }
}

impl Display for Node {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), Error> {
        match self {
            True => formatter.write_str("True"),
            False => formatter.write_str("False"),
            Symbol(symbol) => {
                formatter.write_str("'")?;
                Display::fmt(symbol, formatter)?;
                formatter.write_str("'")
            }
            Text(text) => {
                formatter.write_str("\"")?;
                formatter.write_str(text)?;
                formatter.write_str("\"")
            }
            Define(identifier, node) => {
                formatter.write_str("Define(")?;
                identifier.fmt(formatter)?;
                formatter.write_str(", ")?;
                node.fmt(formatter)?;
                formatter.write_str(")")
            }
            Refer(identifier) => identifier.fmt(formatter),
            Depth(depth, node) => {
                formatter.write_str("Depth(")?;
                Display::fmt(depth, formatter)?;
                formatter.write_str(", ")?;
                node.fmt(formatter)?;
                formatter.write_str(")")
            }
            Store(offset, node) => {
                formatter.write_str("Store(")?;
                Display::fmt(offset, formatter)?;
                formatter.write_str(", ")?;
                node.fmt(formatter)?;
                formatter.write_str(")")
            }
            Spawn(kind) => {
                formatter.write_str("[")?;
                Display::fmt(kind, formatter)?;
                formatter.write_str("]")
            }
            Precede(precedence, bind, node) => {
                formatter.write_str("Precedence(")?;
                Display::fmt(precedence, formatter)?;
                formatter.write_str(", ")?;
                bind.fmt(formatter)?;
                formatter.write_str(", ")?;
                node.fmt(formatter)?;
                formatter.write_str(")")
            }
            And(_, _) => {
                let mut separate = false;
                formatter.write_str("(")?;
                for node in self.flatten() {
                    if mem::replace(&mut separate, true) {
                        formatter.write_str(" & ")?;
                    }
                    node.fmt(formatter)?;
                }
                formatter.write_str(")")
            }
            Or(_, _) => {
                let mut separate = false;
                formatter.write_str("(")?;
                for node in self.flatten() {
                    if mem::replace(&mut separate, true) {
                        formatter.write_str(" | ")?;
                    }
                    node.fmt(formatter)?;
                }
                formatter.write_str(")")
            }
            Switch(cases) => {
                let mut separate = false;
                formatter.write_str("{")?;
                for case in cases {
                    if mem::replace(&mut separate, true) {
                        formatter.write_str(", ")?;
                    }
                    formatter.write_str("'")?;
                    Display::fmt(&case.0, formatter)?;
                    formatter.write_str("'")?;
                    formatter.write_str(": ")?;
                    case.1.fmt(formatter)?;
                }
                formatter.write_str("}")
            }
        }
    }
}

pub fn option(node: impl ToNode) -> Node {
    or(node, True)
}

pub fn store(node: impl ToNode) -> Node {
    Store(0, node.node().into())
}

pub fn or(left: impl ToNode, right: impl ToNode) -> Node {
    Or(left.node().into(), right.node().into())
}

pub fn and(left: impl ToNode, right: impl ToNode) -> Node {
    And(left.node().into(), right.node().into())
}

pub fn any(nodes: Vec<Node>) -> Node {
    let mut nodes = nodes;
    nodes.drain(..).rev().fold(False, |sum, node| or(node, sum))
}

pub fn all(nodes: Vec<Node>) -> Node {
    let mut nodes = nodes;
    nodes.drain(..).rev().fold(True, |sum, node| and(node, sum))
}

pub fn chain(nodes: Vec<Node>) -> Node {
    let mut nodes = nodes;
    nodes
        .drain(..)
        .rev()
        .fold(True, |sum, node| option(and(node, sum)))
}

pub fn repeat(range: impl RangeBounds<usize>, node: impl ToNode) -> Node {
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
        Some(_) => True,
        None => {
            let identifier = Unique(Node::unique());
            let refer = Refer(identifier.clone());
            let node = option(and(node, refer.clone()));
            let define = Define(identifier, node.into());
            and(define, refer)
        }
    };
    and(left, right)
}

pub fn refer(name: &str) -> Node {
    Refer(Path(name.into()))
}

pub fn define(path: &str, node: impl ToNode) -> Node {
    Define(Path(path.into()), node.node().into())
}

pub fn syntax(path: &str, node: impl ToNode) -> Node {
    Define(
        Path(path.into()),
        Depth(1, and(node, Spawn(path.into())).into()).into(),
    )
}

pub fn join(separator: impl ToNode, node: impl ToNode) -> Node {
    let node = node.node();
    option(and(node.clone(), repeat(.., and(separator, node))))
}

pub fn text(text: impl Into<String>) -> Node {
    Text(text.into())
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
