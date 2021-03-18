use crate::node::*;
use std::collections::HashMap;
use std::mem;
use std::rc::Rc;
use Identifier::*;
use Node::*;

/*
    The state can be mutable but must be cloned when facing an 'Or'.
    In the case where 2 branches of an 'Or' succeeds, we know that there is ambiguity in the grammar and the parse
    tree should hold an ambiguity node that points to both results.

    TODO: differentiate between direct and indirect references
        - a direct reference will refer directly to the processor and will not be modifiable
        - an indirect reference will suffer some performance penalty but will be modifiable at runtime

    TODO: try to remove 'String.clone()' especially in the spawn logic
    TODO: collapse pattern ''B' & 'o' & 'b' & 'a'' to a word parser '"Boba"'
    - this parser could use a u128 as a mask to check for multiple characters at once
    TODO: collapse pattern '('A' & A) | ('B' & B) | ('C' & C)' to a map parser { 'A': A, 'B': B, 'C': C }
    TODO: add a range parser?
    TODO: add a state node
    TODO: operator precedence parser

    TODO: retain ambiguities when both branches of an 'Or' succeeds?
    TODO: run each branch of an 'Or' in parallel?
*/

#[derive(Debug, Clone, Default)]
pub struct Tree<'a> {
    pub kind: String, // TODO: try to replace with '&str'
    pub values: Vec<&'a str>,
    pub children: Vec<Tree<'a>>,
}

#[derive(Clone)]
pub struct Parser<'a> {
    root: Parse<'a>,
    references: Vec<Parse<'a>>,
}

#[derive(Clone)]
struct State<'a> {
    pub index: usize,
    pub text: &'a str,
    pub references: Vec<Parse<'a>>,
    pub trees: Vec<(Tree<'a>, isize)>,
    pub precedences: Vec<usize>,
    pub indices: Vec<usize>,
    pub values: Vec<(&'a str, isize)>,
    pub precedence: usize,
    pub depth: isize,
}

type Parse<'a> = Rc<dyn Fn(&mut State<'a>) -> bool + 'a>;

impl<'a> Parser<'a> {
    pub fn parse(&self, text: &'a str) -> Option<Tree<'a>> {
        let mut state = State {
            index: 0,
            text,
            references: self.references.clone(),
            trees: Vec::new(),
            precedences: Vec::new(),
            indices: Vec::new(),
            values: Vec::new(),
            precedence: 0,
            depth: 0,
        };

        println!("Trees");
        println!("{:?}", state.trees);
        if (self.root)(&mut state) && state.index == state.text.len() {
            state.trees.pop().map(|pair| pair.0)
        } else {
            None
        }
    }
}

impl<'a> From<Node> for Parser<'a> {
    fn from(node: Node) -> Parser<'a> {
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

        fn next<'a>(node: &Node, references: &Vec<Option<Parse<'a>>>) -> Parse<'a> {
            match node {
                True => Rc::new(|_| true),
                False => Rc::new(|_| false),
                And(_, _) => {
                    let nodes = node.flatten();
                    let parsers: Vec<_> = nodes.iter().map(|node| next(node, references)).collect();
                    Rc::new(move |state| {
                        for parser in &parsers {
                            if parser(state) {
                                continue;
                            }
                            return false;
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
