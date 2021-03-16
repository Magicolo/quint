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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Bind {
    None,
    Left,
    Right,
}

// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub enum Set {
//     Value(isize),
//     Add(isize),
//     Copy(String),
// }

// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub enum If {
//     Less,
//     Equal,
//     Greater,
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stack {
    Push,
    Pop,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Node {
    True,
    False,
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Define(Identifier, Box<Self>),
    Refer(Identifier),

    Symbol(char),
    Text(String),
    Switch(Vec<(char, Node)>),

    Shift(usize, Box<Self>),
    Spawn(String),
    Depth(isize),
    Precede(usize, Bind, Stack),
    Store(usize, Stack),
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
        /*
            TODO: Apply precedence properly.
            TODO: Find a way to optimize through 'Store/Precedence' nodes.
            TODO: Find a way to reproduce 'push/pop' behavior of 'state.path' that 'Refer' nodes do.
            TODO: This transformation: '{ 'a': b } | (c | { 'd': e }) => { 'a': b, 'c': d } | e'?
                - does not respect order of resolution of '|'
            TODO: Why do stack operations sometimes don't traverse this pattern?
                - Stack(0, Push) & { 'a': b, 'c': d } => { 'a': Stack(1, Push) & b, 'c': Stack(1, Push) & d }
            TODO: Combine 'Depth' some more in 'un_depth':
                - Depth(a) & (Store|Precede) & Depth(b) => (Store|Precede) & Depth(a + b)
        */

        // fn prioritize(node: Node, priority: usize) -> Node {
        //     let (node, priority) = match node {
        //         Precede(current, bind, node) => match bind {
        //             Bind::Left if current <= priority => (False, 0),
        //             Bind::Right if current < priority => (False, 0),
        //             _ => (*node, current),
        //         },
        //         node => (node, priority),
        //     };
        //     node.map(|node| prioritize(node, priority))
        // }

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

        /// (a & b) | (a & c) => a & (b | c)
        fn factor_left(node: Node) -> Node {
            match node {
                And(left, right) => match (*left, *right) {
                    (And(left, middle), right) => {
                        factor_left(And(left, factor_left(And(middle, right.into())).into()))
                    }
                    (left, right) => and(left, right),
                },
                Or(left, right) => match (*left, *right) {
                    (Or(left, middle), right) => {
                        factor_left(Or(left, factor_left(Or(middle, right.into())).into()))
                    }
                    (And(left1, right1), And(mut left2, right2)) if left1 == left2 => {
                        *left2 = factor_left(Or(right1, right2));
                        factor_left(And(left1, left2))
                    }
                    (left, right) => or(left, right),
                },
                node => node,
            }
        }

        /// (a & c) | (b & c) => (a | b) & c
        fn factor_right(node: Node) -> Node {
            match node {
                And(left, right) => match (*left, *right) {
                    (left, And(middle, right)) => {
                        factor_right(And(factor_right(And(left.into(), middle)).into(), right))
                    }
                    (left, right) => and(left, right),
                },
                Or(left, right) => match (*left, *right) {
                    (left, Or(middle, right)) => {
                        factor_right(Or(factor_right(Or(left.into(), middle)).into(), right))
                    }
                    (And(left1, mut right1), And(left2, right2)) if right1 == right2 => {
                        *right1 = factor_right(Or(left1, left2));
                        factor_right(And(right1, right2))
                    }
                    (left, right) => or(left, right),
                },
                node => node,
            }
        }

        /// True & a => a, a & True => a, False & a => False, a & False => False,
        /// a | a => a, False | a => a, a | False => a, True | a => a | True
        fn boolean(node: Node) -> Node {
            match node {
                And(left, right) => match (*left, *right) {
                    (True, right) => right,
                    (left, True) => left,
                    (False, _) => False,
                    (_, False) => False,
                    (left, right) => and(left, right),
                },
                Or(left, right) => match (*left, *right) {
                    (left, right) if left == right => left,
                    (False, right) => right,
                    (left, False) => left,
                    (True, right) => or(right, True),
                    (left, right) => or(left, right),
                },
                node => node,
            }
        }

        fn shift_right(node: Node) -> Node {
            match node {
                And(left, right) => match (*left, *right) {
                    (And(left, middle), right) => {
                        And(left, shift_right(And(middle, right.into())).into())
                    }
                    (Shift(shift, node), True) => shift_right(and(True, Shift(shift, node))),
                    (Shift(shift, node), Symbol(symbol)) => {
                        let shift = shift + symbol.len_utf8();
                        shift_right(and(Symbol(symbol), Shift(shift, node)))
                    }
                    (Shift(shift, node), Text(text)) => {
                        let shift = shift + text.len();
                        shift_right(and(Text(text), Shift(shift, node)))
                    }
                    (Shift(shift, node), And(left, right)) => shift_right(And(
                        shift_right(And(Shift(shift, node).into(), left)).into(),
                        right,
                    )),
                    (Shift(shift, node), Or(left, right)) => shift_right(factor_right(or(
                        shift_right(and(Shift(shift, node.clone()), left)),
                        shift_right(and(Shift(shift, node), right)),
                    ))),
                    (left, right) => and(left, right),
                },
                Spawn(kind) => Shift(0, Spawn(kind).into()),
                Depth(depth) => Shift(0, Depth(depth).into()),
                Store(shift, stack) => Shift(0, Store(shift, stack).into()),
                Precede(precedence, bind, stack) => {
                    Shift(0, Precede(precedence, bind, stack).into())
                }
                node => node,
            }
        }

        fn un_shift(node: Node) -> Node {
            match node {
                Shift(shift, node) => match *node {
                    Store(inner, action) => Store(shift + inner, action),
                    node => node,
                },
                node => node,
            }
        }

        fn un_depth(node: Node) -> Node {
            match node {
                And(left, right) => match (*left, *right) {
                    (Depth(left), Depth(right)) => Depth(left + right),
                    (Depth(left), And(middle, right)) => match *middle {
                        Depth(middle) => And(Depth(left + middle).into(), right),
                        middle => and(Depth(left), And(middle.into(), right)),
                    },
                    (left, right) => and(left, right),
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

        /// { 'a': b } | ({ 'c': d } | e) => { 'a': b, 'c': d } | e
        fn process(node: Node) -> Node {
            match boolean(node) {
                And(left, right) => match (*left, *right) {
                    (And(left, middle), right) => {
                        process(And(left, process(And(middle, right.into())).into()))
                    }
                    (Switch(mut left), right) => {
                        for case in left.iter_mut() {
                            let value = mem::replace(&mut case.1, True);
                            case.1 = process(and(value, right.clone()));
                        }
                        process(Switch(left))
                    }
                    (left, right) => and(left, right),
                },
                Or(left, right) => match (*left, *right) {
                    (Or(left, middle), right) => {
                        process(Or(left, process(Or(middle, right.into())).into()))
                    }
                    (Switch(mut left), Or(middle, right)) => match (*middle, *right) {
                        (Switch(mut middle), right) => {
                            left.append(&mut middle);
                            process(or(Switch(left), right))
                        }
                        (middle, right) => or(Switch(left), or(middle, right)),
                    },
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
            match boolean(node) {
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
                    post(and(Text(case.0.into()), case.1))
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

        fn optimize<T: Clone>(
            node: Node,
            context: &mut Context<T>,
            set: &mut HashSet<usize>,
        ) -> Node {
            node.descend(|node| expand(node, context, set))
                .descend(shift_right)
                .descend(factor_left)
                .descend(un_shift)
                .descend(un_depth)
                .descend(pre)
                .descend(process)
                .descend(post)
        }

        println!("Original: {}", node.count());
        println!("{}", node);
        let node = node.descend(|node| identify(node, self));
        let node = optimize(node, self, &mut HashSet::new());
        println!("Optimize: {}", node.count());
        println!("{}", node);
        for pair in self.definitions.iter() {
            println!("{}: {} => {}", pair.0, pair.1.count(), pair.1);
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

    pub fn count(&self) -> usize {
        match self {
            And(left, right) => left.count() + right.count() + 1,
            Or(left, right) => left.count() + right.count() + 1,
            Define(_, node) => node.count() + 1,
            Shift(_, node) => node.count() + 1,
            Switch(cases) => cases.iter().fold(1, |count, case| count + case.1.count()),
            _ => 1,
        }
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
            Shift(shift, mut node) => {
                *node = map(*node);
                Shift(shift, node)
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
            Spawn(kind) => {
                formatter.write_str("[")?;
                Display::fmt(kind, formatter)?;
                formatter.write_str("]")
            }
            Depth(depth) => {
                formatter.write_str("Depth(")?;
                Display::fmt(depth, formatter)?;
                formatter.write_str(")")
            }
            Store(shift, stack) => {
                let stack = match stack {
                    Stack::Push => "+",
                    Stack::Pop => "-",
                };
                formatter.write_str(stack)?;
                formatter.write_str("Store(")?;
                Display::fmt(shift, formatter)?;
                formatter.write_str(")")?;
                formatter.write_str(stack)
            }
            Precede(precedence, bind, stack) => {
                let stack = match stack {
                    Stack::Push => "+",
                    Stack::Pop => "-",
                };
                formatter.write_str(stack)?;
                formatter.write_str("Precede(")?;
                Display::fmt(precedence, formatter)?;
                formatter.write_str(", ")?;
                Debug::fmt(bind, formatter)?;
                formatter.write_str(")")?;
                formatter.write_str(stack)
            }
            Shift(shift, node) => {
                node.fmt(formatter)?;
                formatter.write_str("<")?;
                Display::fmt(shift, formatter)?;
                formatter.write_str(">")
            }
        }
    }
}

pub fn option(node: impl ToNode) -> Node {
    or(node, True)
}

pub fn store(node: impl ToNode) -> Node {
    and(Store(0, Stack::Push), and(node, Store(0, Stack::Pop)))
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
        and(Depth(1), and(node, and(Depth(-1), Spawn(path.into())))).into(),
    )
}

pub fn join(separator: impl ToNode, node: impl ToNode) -> Node {
    let node = node.node();
    option(and(node.clone(), repeat(.., and(separator, node))))
}

pub fn text(text: impl Into<String>) -> Node {
    Text(text.into())
}

pub fn prefix(precedence: usize, node: impl ToNode) -> Node {
    and(
        Precede(precedence, Bind::None, Stack::Push),
        and(node, Precede(precedence, Bind::None, Stack::Pop)),
    )
}

pub fn postfix(precedence: usize, bind: Bind, node: impl ToNode) -> Node {
    and(
        Precede(precedence, bind, Stack::Push),
        and(node, Precede(precedence, bind, Stack::Pop)),
    )
}

pub fn precede(prefix: impl ToNode, postfix: impl ToNode) -> Node {
    and(prefix, repeat(.., postfix))
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
