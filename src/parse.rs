use crate::node::*;
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
    pub values: Vec<(&'a str, isize)>,
    pub precedence: usize,
    pub depth: isize,
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
            values: Vec::new(),
            precedence: 0,
            depth: 0,
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

        fn next(node: &Node, references: &Vec<Option<Parse>>) -> Parse {
            match node {
                True => Rc::new(|_| true),
                False => Rc::new(|_| false),
                And(_, _) => {
                    let nodes = node.flatten();
                    let parsers: Vec<_> = nodes.iter().map(|node| next(node, references)).collect();
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
                    let parsers: Vec<_> = nodes.iter().map(|node| next(node, references)).collect();
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
                Refer(Index(index)) => match &references[*index] {
                    Some(parser) => parser.clone(),
                    None => {
                        let index = *index;
                        Rc::new(move |state| state.references[index].clone()(state))
                    }
                },
                Spawn(kind) => {
                    let kind = kind.clone();
                    Rc::new(move |state| {
                        let tree = Tree {
                            kind: kind.clone(),
                            values: consume(&mut state.values, state.depth),
                            children: consume(&mut state.trees, state.depth),
                        };
                        state.trees.push((tree, state.depth));
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
                Depth(depth) => {
                    let depth = *depth;
                    Rc::new(move |state| {
                        state.depth += depth;
                        true
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
                    let shift = *shift;
                    Rc::new(move |state| match state.indices.pop() {
                        Some(index) => {
                            let value = &state.text[index..state.index - shift];
                            state.values.push((value, state.depth));
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
                Switch(cases) => {
                    let mut map = HashMap::new();
                    for case in cases {
                        map.insert(case.0, next(&case.1, references));
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

        let (node, nodes) = node.resolve();
        let mut references = vec![None; nodes.len()];
        for i in 0..nodes.len() {
            references[i] = Some(next(&nodes[i], &references));
        }
        let root = next(&node, &references);
        let references = references.drain(..).map(|parser| parser.unwrap()).collect();
        Parser { root, references }
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
