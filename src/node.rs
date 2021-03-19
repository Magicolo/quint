use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::fmt::{Display, Error, Formatter};
use std::hash::Hash;
use std::mem;
use std::ops::Range;
use std::ops::RangeInclusive;
use std::ops::{Bound, RangeBounds};
use std::str;
use std::sync::atomic::{AtomicUsize, Ordering};
use Identifier::*;
use Node::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Identifier {
    Unique(usize),
    Index(usize),
    Path(String),
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
pub enum Bind {
    None,
    Left,
    Right,
}

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

impl ToNode for char {
    fn node(self) -> Node {
        text(self)
    }
}

impl ToNode for &str {
    fn node(self) -> Node {
        text(self)
    }
}

impl ToNode for String {
    fn node(self) -> Node {
        text(self)
    }
}

impl ToNode for Range<char> {
    fn node(self) -> Node {
        range(self.start, self.end)
    }
}

impl ToNode for RangeInclusive<char> {
    fn node(self) -> Node {
        range(*self.start(), *self.end())
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
            Switch(cases) => cases
                .iter()
                .fold(1, |count, case| count + case.1.count() + 1),
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

    pub fn resolve(self) -> (Node, Vec<Node>) {
        struct State {
            nodes: Vec<Option<Node>>,
            references: HashMap<Node, usize>,
            indices: HashMap<Identifier, usize>,
            optimize: HashSet<usize>,
            refer_threshold: usize,
        }

        /*
            TODO: Apply precedence properly.
            TODO: This transformation: '{ 'a': b } | (c | { 'd': e }) => { 'a': b, 'c': d } | e'?
                - requires to commit to unordered '|'
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

        fn normalize(node: Node) -> Node {
            match boolean(node) {
                And(left, right) => match (*left, *right) {
                    (And(left, middle), right) => And(left, And(middle, right.into()).into()),
                    (left, right) => and(left, right),
                },
                Or(left, right) => match (*left, *right) {
                    (Or(left, middle), right) => Or(left, Or(middle, right.into()).into()),
                    (left, right) => or(left, right),
                },
                Switch(mut cases) => any(cases.drain(..).map(|pair| and(pair.0, pair.1)).collect())
                    .descend(normalize),
                Text(text) => all(text.chars().map(Symbol).collect()).descend(normalize),
                node => node,
            }
        }

        fn index(identifier: Identifier, state: &mut State) -> usize {
            match identifier {
                Index(index) => index,
                identifier => match state.indices.get(&identifier) {
                    Some(index) => *index,
                    None => {
                        let index = state.nodes.len();
                        state.nodes.push(None);
                        state.indices.insert(identifier, index);
                        index
                    }
                },
            }
        }

        fn define(identifier: Identifier, node: Node, state: &mut State) -> usize {
            match (state.references.get(&node), identifier) {
                (Some(index), _) => *index,
                (None, Path(path)) => {
                    let mut parts: Vec<_> = path.split(".").collect();
                    while parts.len() > 0 {
                        let index = index(Path(parts.join(".")), state);
                        match mem::replace(&mut state.nodes[index], None) {
                            Some(left) => state.nodes[index] = Some(or(left, node.clone())),
                            None => state.nodes[index] = Some(node.clone()),
                        }
                        parts.pop();
                    }
                    index(Path(path), state)
                }
                (None, identifier) => {
                    let index = index(identifier, state);
                    state.nodes[index] = Some(node.clone());
                    state.references.insert(node, index);
                    index
                }
            }
        }

        fn refer(node: Node, state: &mut State) -> Node {
            Refer(Index(define(Unique(Node::unique()), node, state)))
        }

        fn identify(node: Node, state: &mut State) -> Node {
            match boolean(node) {
                Define(identifier, node) => {
                    define(identifier, *node, state);
                    True
                }
                Refer(identifier) => Refer(Index(index(identifier, state))),
                // node if node.count() > 8 => refer(node, state),
                node => node,
            }
        }

        fn expand(node: Node, state: &mut State) -> Node {
            fn next(node: Node, state: &mut State) -> Node {
                match node {
                    Refer(Index(index)) if state.optimize.insert(index) => {
                        let node = state.nodes[index].clone().unwrap_or(False);
                        let node = optimize(node, state);
                        state.nodes[index] = Some(node.clone());
                        node
                    }
                    Refer(Index(index)) => state.nodes[index].clone().unwrap_or(False),
                    node => node.map(|node| next(node, state)),
                }
            }

            next(node, state)
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
            match boolean(node) {
                And(left, right) => match (*left, *right) {
                    (And(left, middle), right) => {
                        And(left, shift_right(And(middle, right.into())).into())
                    }
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

        /// { 'a': b, 'c': d } & e => { 'a': b & e, 'c': d & e }
        /// { 'a': b } | ({ 'c': d } | e) => { 'a': b, 'c': d } | e
        fn process(node: Node, state: &mut State) -> Node {
            match boolean(node) {
                And(left, right) => match (*left, *right) {
                    (And(left, middle), right) => process(
                        And(left, process(And(middle, right.into()), state).into()),
                        state,
                    ),
                    (Switch(mut left), right) => {
                        let node = if left.len() <= 1
                            || left.len() * right.count() <= state.refer_threshold
                        {
                            right
                        } else {
                            // If the cloning of the 'right' node would cause an explosion in nodes,
                            // create a reference instead. In that case, the optimization must be
                            // manually completed for the node.
                            refer(right.descend(post), state)
                        };

                        for case in left.iter_mut() {
                            let value = mem::replace(&mut case.1, True);
                            case.1 = process(and(value, node.clone()), state);
                        }
                        Switch(left)
                    }
                    (left, right) => and(left, right),
                },
                Or(left, right) => match (*left, *right) {
                    (Or(left, middle), right) => process(
                        Or(left, process(Or(middle, right.into()), state).into()),
                        state,
                    ),
                    (Switch(mut left), Or(middle, right)) => match *middle {
                        Switch(mut middle) => {
                            left.append(&mut middle);
                            process(Or(Switch(left).into(), right), state)
                        }
                        middle => or(Switch(left), Or(middle.into(), right)),
                    },
                    (Switch(mut left), Switch(mut right)) => {
                        left.append(&mut right);
                        process(Switch(left), state)
                    }
                    (left, right) => or(left, right),
                },
                Switch(cases) if cases.len() == 0 => True,
                Switch(mut cases) => {
                    let mut map = HashMap::new();
                    for case in cases.drain(..) {
                        match map.get_mut(&case.0) {
                            Some(value) => {
                                *value = process(or(mem::replace(value, True), case.1), state);
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

        fn optimize(node: Node, state: &mut State) -> Node {
            expand(node, state)
                .descend(shift_right)
                .descend(factor_left)
                .descend(un_shift)
                .descend(un_depth)
                .descend(pre)
                .descend(|node| process(node, state))
                .descend(post)
        }

        fn print(title: &str, node: &Node, state: &State) {
            println!();
            println!("{}: {}", title, node);
            println!(
                "{}",
                state
                    .nodes
                    .iter()
                    .enumerate()
                    .map(|pair| format!("{} => {}", pair.0, pair.1.as_ref().unwrap_or(&True)))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            println!("{:?}, {:?}", state.optimize, state.indices);
        }

        let mut state = State {
            nodes: Vec::new(),
            references: HashMap::new(),
            indices: HashMap::new(),
            optimize: HashSet::new(),
            refer_threshold: 1024,
        };
        // print("ORIGINAL", &self, &state);
        let node = self
            .descend(normalize)
            .descend(|node| identify(node, &mut state));
        // print("IDENTIFY", &node, &state);
        let node = optimize(node, &mut state);
        // print("OPTIMIZE", &node, &state);

        let nodes = state
            .nodes
            .drain(..)
            .map(|node| node.unwrap_or(False))
            .collect();
        (node, nodes)
    }
}

impl Debug for Node {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
        Display::fmt(self, formatter)
    }
}

impl Display for Node {
    fn fmt(&self, formatter: &mut Formatter) -> Result<(), Error> {
        match self {
            True => formatter.write_str("True"),
            False => formatter.write_str("False"),
            Symbol(symbol) => {
                formatter.write_str("'")?;
                Display::fmt(&symbol.escape_debug(), formatter)?;
                formatter.write_str("'")
            }
            Text(text) => {
                formatter.write_str("\"")?;
                formatter.write_str(text.escape_debug().collect::<String>().as_str())?;
                formatter.write_str("\"")
            }
            Define(identifier, node) => {
                formatter.write_str("Define(")?;
                identifier.fmt(formatter)?;
                formatter.write_str(", ")?;
                Display::fmt(node, formatter)?;
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
                    Display::fmt(node, formatter)?;
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
                    Display::fmt(node, formatter)?;
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
                    Display::fmt(&case.0.escape_debug(), formatter)?;
                    formatter.write_str("'")?;
                    formatter.write_str(": ")?;
                    Display::fmt(&case.1, formatter)?;
                }
                formatter.write_str("}")
            }
            Spawn(kind) => {
                formatter.write_str("[")?;
                Display::fmt(kind, formatter)?;
                formatter.write_str("]")
            }
            Depth(depth) => {
                formatter.write_str("D(")?;
                Display::fmt(depth, formatter)?;
                formatter.write_str(")")
            }
            Store(shift, stack) => {
                let stack = match stack {
                    Stack::Push => "+",
                    Stack::Pop => "-",
                };
                formatter.write_str(stack)?;
                formatter.write_str("S(")?;
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
                Display::fmt(node, formatter)?;
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
            let identifier = Identifier::Unique(Node::unique());
            let refer = Refer(identifier.clone());
            let node = option(and(node, refer.clone()));
            let define = Define(identifier, node.into());
            and(define, refer)
        }
    };
    and(left, right)
}

pub fn refer(name: &str) -> Node {
    Refer(Identifier::Path(name.into()))
}

pub fn define(path: &str, node: impl ToNode) -> Node {
    Define(Identifier::Path(path.into()), node.node().into())
}

pub fn syntax(path: &str, node: impl ToNode) -> Node {
    Define(
        Identifier::Path(path.into()),
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

pub fn range(low: char, high: char) -> Node {
    any((low as u8..=high as u8)
        .into_iter()
        .map(|index| text(index as char))
        .collect())
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
