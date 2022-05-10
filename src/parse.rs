use crate::node::*;
use crate::node::{If, Set};
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Error;
use std::fmt::Formatter;
use std::mem;
use std::rc::Rc;
use Identifier::*;
use Node::*;

/*
    TODO: try to remove 'String.clone()' especially in the spawn logic
    TODO: add state nodes
    TODO: operator precedence parser
    TODO: retain ambiguities when both branches of an 'Or' succeeds?
*/

#[derive(Clone, Default)]
pub struct Tree<'a> {
    pub kind: String,
    pub values: Vec<&'a str>,
    pub children: Vec<Tree<'a>>,
}

#[derive(Clone)]
pub struct Parser {
    root: Parse,
    references: Vec<Parse>,
    indices: HashMap<Identifier, usize>,
}

type Parse = Rc<dyn Fn(&mut State) -> bool>;

#[derive(Clone)]
struct State<'a, 'b> {
    pub index: usize,
    pub text: &'a str,
    pub references: &'b Vec<Parse>,
    pub trees: Vec<(Tree<'a>, isize)>,
    pub precedences: Vec<usize>,
    pub indices: Vec<usize>,
    pub stores: Vec<(&'a str, isize)>,
    pub precedence: usize,
    pub values: Vec<isize>,
}

impl Parser {
    pub fn parse<'a>(&self, text: &'a str) -> Vec<Tree<'a>> {
        let mut state = State {
            index: 0,
            text,
            references: &self.references,
            trees: Vec::new(),
            precedences: Vec::new(),
            indices: Vec::new(),
            stores: Vec::new(),
            precedence: 0,
            values: vec![0; self.indices.len()],
        };

        if (self.root)(&mut state) && state.index == state.text.len() {
            state.trees.drain(..).map(|pair| pair.0).collect()
        } else {
            Vec::new()
        }
    }
}

impl From<Node> for Parser {
    fn from(node: Node) -> Parser {
        struct State {
            depth: usize,
            references: Vec<Option<Parse>>,
        }

        fn consume<T>(pairs: &mut Vec<(T, isize)>, depth: isize) -> Vec<T> {
            let mut values = Vec::new();
            while let Some(pair) = pairs.pop() {
                if pair.1 > depth {
                    values.push(pair.0);
                } else {
                    pairs.push(pair);
                    break;
                }
            }
            values.reverse();
            values
        }

        fn next(node: &Node, state: &State) -> Parse {
            match node {
                True => Rc::new(|_| true),
                False => Rc::new(|_| false),
                And(_, _) => {
                    let nodes = node.flatten();
                    let parsers: Vec<_> = nodes.iter().map(|node| next(node, state)).collect();
                    Rc::new(move |state| {
                        for parser in &parsers {
                            if parser(state) == false {
                                return false;
                            }
                        }
                        true
                    })
                }
                Or(_, _) => {
                    let nodes = node.flatten();
                    let parsers: Vec<_> = nodes.iter().map(|node| next(node, state)).collect();
                    Rc::new(move |state| {
                        for parser in &parsers {
                            let mut local = state.clone();
                            if parser(&mut local) {
                                *state = local;
                                return true;
                            }
                        }
                        false
                    })
                }
                Refer(Index(index)) => match &state.references[*index] {
                    Some(parser) => parser.clone(),
                    None => {
                        let index = *index;
                        Rc::new(move |state| state.references[index].clone()(state))
                    }
                },
                Spawn(kind) => {
                    let depth = state.depth;
                    let kind = kind.clone();
                    Rc::new(move |state| {
                        let depth = state.values[depth];
                        let tree = Tree {
                            kind: kind.clone(),
                            values: consume(&mut state.stores, depth),
                            children: consume(&mut state.trees, depth),
                        };
                        state.trees.push((tree, depth));
                        true
                    })
                }
                Symbol(symbol) => {
                    let symbol = *symbol;
                    Rc::new(move |state| match state.text.get(state.index..) {
                        Some(slice) if slice.starts_with(symbol) => {
                            state.index += symbol.len_utf8();
                            true
                        }
                        _ => false,
                    })
                }
                Text(text) => {
                    let text = text.clone();
                    Rc::new(move |state| match state.text.get(state.index..) {
                        Some(slice) if slice.starts_with(text.as_str()) => {
                            state.index += text.len();
                            true
                        }
                        _ => false,
                    })
                }
                Store(shift, Stack::Push) => {
                    let shift = *shift;
                    Rc::new(move |state| {
                        state.indices.push(state.index - shift);
                        true
                    })
                }
                Store(shift, Stack::Pop) => {
                    let depth = state.depth;
                    let shift = *shift;
                    Rc::new(move |state| match state.indices.pop() {
                        Some(index) => {
                            let depth = state.values[depth];
                            let value = &state.text[index..state.index - shift];
                            state.stores.push((value, depth));
                            true
                        }
                        None => false,
                    })
                }
                Precede(precedence, bind, Stack::Push) => {
                    let precedence = *precedence;
                    let bind = bind.clone();
                    Rc::new(move |state| match bind {
                        Bind::Left if precedence <= state.precedence => false,
                        Bind::Right if precedence < state.precedence => false,
                        _ => {
                            let precedence = mem::replace(&mut state.precedence, precedence);
                            state.precedences.push(precedence);
                            true
                        }
                    })
                }
                Precede(_, _, Stack::Pop) => Rc::new(move |state| match state.precedences.pop() {
                    Some(precedence) => {
                        state.precedence = precedence;
                        true
                    }
                    None => false,
                }),
                Set(Index(index), Set::Copy(Index(copy))) => {
                    let index = *index;
                    let copy = *copy;
                    Rc::new(move |state| {
                        state.values[index] = state.values[copy];
                        true
                    })
                }
                Set(Index(index), Set::Add(value)) => {
                    let index = *index;
                    let value = *value;
                    Rc::new(move |state| {
                        state.values[index] += value;
                        true
                    })
                }
                Set(Index(index), Set::Value(value)) => {
                    let index = *index;
                    let value = *value;
                    Rc::new(move |state| {
                        state.values[index] = value;
                        true
                    })
                }
                If(Index(left), If::Less, Index(right)) => {
                    let left = *left;
                    let right = *right;
                    Rc::new(move |state| state.values[left] < state.values[right])
                }
                If(Index(left), If::Equal, Index(right)) => {
                    let left = *left;
                    let right = *right;
                    Rc::new(move |state| state.values[left] == state.values[right])
                }
                Switch(cases) => {
                    let mut map = HashMap::new();
                    for case in cases {
                        map.insert(case.0, next(&case.1, state));
                    }

                    Rc::new(move |state| {
                        match state
                            .text
                            .get(state.index..)
                            .and_then(|text| text.chars().next())
                            .map(|key| (key, map.get(&key)))
                        {
                            Some((key, Some(parser))) => {
                                state.index += key.len_utf8();
                                parser(state)
                            }
                            _ => false,
                        }
                    })
                }
                node => panic!("Invalid node '{}'.", node),
            }
        }

        let (node, nodes, _, mut indices) = node.resolve();
        let depth = Path(".depth".into());
        let depth_index = match indices.get(&depth) {
            Some(index) => *index,
            None => {
                let index = indices.len();
                indices.insert(depth, index);
                index
            }
        };

        let mut state = State {
            depth: depth_index,
            references: vec![None; nodes.len()],
        };
        for i in 0..nodes.len() {
            state.references[i] = Some(next(&nodes[i], &state));
        }
        let root = next(&node, &state);
        let references = state
            .references
            .drain(..)
            .map(|parser| parser.unwrap())
            .collect();
        Parser {
            root,
            references,
            indices,
        }
    }
}

impl Debug for Tree<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
        Display::fmt(self, formatter)
    }
}

impl Display for Tree<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
        formatter.write_str(&format!("{}", self.kind))?;
        if self.values.len() > 0 {
            let values = self
                .values
                .iter()
                .map(|value| format!(r#""{}""#, value))
                .collect::<Vec<_>>()
                .join(", ");
            formatter.write_str(&format!("({})", values))?;
        }
        if self.children.len() > 0 {
            let children = self
                .children
                .iter()
                .map(|child| child.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            formatter.write_str(&format!(": {} {} {}", "{", children, "}"))?;
        }
        Ok(())
    }
}
