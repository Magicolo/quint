use crate::node::*;
use std::collections::HashMap;
use std::mem;
use std::ops::{Range, RangeInclusive};
use std::rc::Rc;
use Identifier::*;
use Node::*;

#[derive(Debug, Clone, Default)]
pub struct Tree<'a> {
    pub kind: String, // TODO: try to replace with '&str'
    pub value: &'a str,
    pub children: Vec<Tree<'a>>,
}

#[derive(Debug, Clone, Default)]
pub struct State<'a> {
    pub index: usize,
    pub text: &'a str,
    pub path: String, // TODO: try to replace with '&str'
    pub trees: Vec<(Tree<'a>, usize)>,
    pub precedence: usize,
    pub depth: usize,
    pub offset: usize,
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

#[derive(Clone)]
pub struct Parser<'a>(Rc<Parse<'a>>);
type Parse<'a> = dyn Fn(&mut State<'a>, &Context<Parser<'a>>) -> bool + 'a;

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

fn parsers<'a>(node: Node) -> (Parser<'a>, Context<Parser<'a>>) {
    fn next<'a>(node: &Node, parsers: &HashMap<usize, Parser<'a>>) -> Parser<'a> {
        match node {
            True => Parser(Rc::new(|_, _| true)),
            False => Parser(Rc::new(|_, _| false)),
            And(_, _) => {
                let nodes = node.flatten();
                let parsers: Vec<_> = nodes.iter().map(|node| next(node, parsers)).collect();
                Parser(Rc::new(move |state, context| {
                    for parser in &parsers {
                        if parser.0(state, context) {
                            continue;
                        }
                        return false;
                    }
                    true
                }))
            }
            Or(_, _) => {
                let nodes = node.flatten();
                let parsers: Vec<_> = nodes.iter().map(|node| next(node, parsers)).collect();
                Parser(Rc::new(move |state, context| {
                    for parser in &parsers {
                        let mut local = state.clone();
                        if parser.0(&mut local, context) {
                            *state = local;
                            return true;
                        }
                    }
                    false
                }))
            }
            Refer(Unique(identifier)) => match parsers.get(&identifier) {
                Some(parser) => parser.clone(),
                None => {
                    let identifier = *identifier;
                    Parser(Rc::new(move |state, context| {
                        context.references[&identifier].0(state, context)
                    }))
                }
            },
            // Refer(Path(path)) => {
            //     let identifier = context.identify(&Path(path.clone()));
            //     let path = path.clone();
            //     Parser(Rc::new(move |state, context| {
            //         match context.references.get(&identifier) {
            //             Some(parser) => {
            //                 let path = mem::replace(&mut state.path, path.clone());
            //                 let result = parser.0(state, context);
            //                 state.path = path;
            //                 result
            //             }
            //             None => false,
            //         }
            //     }))
            // }
            // Spawn(node) => {
            //     let parser = next(node, parsers);
            //     Parser(Rc::new(move |state, context| {
            //         let index = state.index;
            //         let parents = mem::replace(&mut state.trees, Vec::new());
            //         if parser.0(state, context) {
            //             let children = mem::replace(&mut state.trees, parents);
            //             state.trees.push(Tree {
            //                 kind: state.path.clone(),
            //                 value: &state.text[index..state.index],
            //                 children,
            //             });
            //             true
            //         } else {
            //             false
            //         }
            //     }))
            // }
            Depth(depth, node) => {
                let depth = *depth;
                let parser = next(node, parsers);
                Parser(Rc::new(move |state, context| {
                    state.depth += depth;
                    let result = parser.0(state, context);
                    state.depth -= depth;
                    result
                }))
            }
            Store(offset, node) => {
                let offset = *offset;
                let parser = next(node, parsers);
                Parser(Rc::new(move |state, context| {
                    state.offset += offset;
                    let result = parser.0(state, context);
                    state.offset -= offset;
                    result
                }))
            }
            Spawn(kind) => {
                let kind = kind.clone();
                Parser(Rc::new(move |state, _| {
                    let mut children = Vec::new();
                    while let Some(pair) = state.trees.pop() {
                        if pair.1 > state.depth {
                            children.push(pair.0);
                        } else {
                            state.trees.push(pair);
                            break;
                        }
                    }
                    children.reverse();

                    let tree = Tree {
                        kind: kind.clone(),
                        value: &state.text[state.index - state.offset - 1..state.index],
                        children,
                    };
                    state.trees.push((tree, state.depth));
                    true
                }))
            }
            Symbol(symbol) => {
                let symbol = *symbol;
                Parser(Rc::new(move |state, _| {
                    match state.text.get(state.index..) {
                        Some(slice) if slice.starts_with(symbol) => {
                            state.index += symbol.len_utf8();
                            true
                        }
                        _ => false,
                    }
                }))
            }
            Text(text) => {
                let text = text.clone();
                Parser(Rc::new(move |state, _| {
                    match state.text.get(state.index..) {
                        Some(slice) if slice.starts_with(text.as_str()) => {
                            state.index += text.len();
                            true
                        }
                        _ => false,
                    }
                }))
            }
            Precede(precedence, bind, node) => {
                let precedence = *precedence;
                let bind = bind.clone();
                let parser = next(node, parsers);
                Parser(Rc::new(move |state, context| match bind {
                    Bind::Left if precedence <= state.precedence => false,
                    Bind::Right if precedence < state.precedence => false,
                    _ => {
                        let precedence = mem::replace(&mut state.precedence, precedence);
                        let result = parser.0(state, context);
                        state.precedence = precedence;
                        result
                    }
                }))
            }
            Switch(cases) => {
                let mut map = HashMap::new();
                for case in cases {
                    map.insert(case.0, next(&case.1, parsers));
                }

                Parser(Rc::new(move |state, context| {
                    match state
                        .text
                        .get(state.index..)
                        .and_then(|text| text.chars().next())
                        .map(|key| (key, map.get(&key)))
                    {
                        Some((key, Some(parser))) => {
                            state.index += key.len_utf8();
                            parser.0(state, context)
                        }
                        _ => false,
                    }
                }))
            }
            _ => panic!("Invalid node '{}'.", node),
        }
    }

    let mut context = Context::new();
    let mut parsers = HashMap::new();
    let node = context.resolve(node);
    for (identifier, node) in &context.definitions {
        let parser = next(node, &parsers);
        parsers.insert(*identifier, parser);
    }
    context.references = parsers;
    (next(&node, &context.references), context)
}

pub fn parse<'a>(text: &'a str, node: Node) -> Option<Tree<'a>> {
    let (parser, context) = parsers(node);
    let mut state = State::default();
    state.text = text;

    println!("Trees");
    println!("{:?}", state.trees);
    if parser.0(&mut state, &context) && state.index == state.text.len() {
        state.trees.pop().map(|pair| pair.0)
    } else {
        None
    }
}

pub fn prefix(precedence: usize, node: impl ToNode) -> Node {
    Precede(precedence, Bind::None, node.node().into())
}

pub fn postfix(precedence: usize, bind: Bind, node: impl ToNode) -> Node {
    Precede(precedence, bind, node.node().into())
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
