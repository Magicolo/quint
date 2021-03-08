use crate::node::*;
use std::mem;
use std::ops::{Range, RangeInclusive};
use std::rc::Rc;

#[derive(Debug, Clone, Default)]
pub struct Tree<'a> {
    pub kind: String, // TODO: try to replace with '&str'
    pub value: &'a str,
    pub children: Vec<Tree<'a>>,
}

#[derive(Debug, Clone)]
pub struct State<'a> {
    pub index: usize,
    pub text: &'a str,
    pub path: String, // TODO: try to replace with '&str'
    pub trees: Vec<Tree<'a>>,
    pub precedence: usize,
}

impl ToNode for char {
    fn node(self) -> Node {
        Node::Symbol(self)
    }
}

impl ToNode for &str {
    fn node(self) -> Node {
        all(self.chars().map(symbol).collect())
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
    fn next<'a>(node: &Node, context: &mut Context<Parser<'a>>) -> Parser<'a> {
        match node {
            Node::True => Parser(Rc::new(|_, _| true)),
            Node::False => Parser(Rc::new(|_, _| false)),
            Node::And(_, _) => {
                let nodes = node.flatten();
                let parsers: Vec<_> = nodes.iter().map(|node| next(node, context)).collect();
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
            Node::Or(_, _) => {
                let nodes = node.flatten();
                let parsers: Vec<_> = nodes.iter().map(|node| next(node, context)).collect();
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
            Node::Define(identifier, node) => {
                let parser = next(node, context);
                context.refer(identifier, parser);
                next(&Node::True, context)
            }
            Node::Refer(Identifier::Unique(identifier)) => {
                let identifier = *identifier;
                Parser(Rc::new(move |state, context| {
                    match context.references.get(&identifier) {
                        Some(parser) => parser.0(state, context),
                        None => false,
                    }
                }))
            }
            Node::Refer(Identifier::Path(path)) => {
                let identifier = context.identify(&Identifier::Path(path.clone()));
                let path = path.clone();
                Parser(Rc::new(move |state, context| {
                    match context.references.get(&identifier) {
                        Some(parser) => {
                            let path = mem::replace(&mut state.path, path.clone());
                            let result = parser.0(state, context);
                            state.path = path;
                            result
                        }
                        None => false,
                    }
                }))
            }
            Node::Spawn(node) => {
                let parser = next(node, context);
                Parser(Rc::new(move |state, context| {
                    let index = state.index;
                    let parents = mem::replace(&mut state.trees, Vec::new());
                    if parser.0(state, context) {
                        let children = mem::replace(&mut state.trees, parents);
                        state.trees.push(Tree {
                            kind: state.path.clone(),
                            value: &state.text[index..state.index],
                            children,
                        });
                        true
                    } else {
                        false
                    }
                }))
            }
            Node::Symbol(symbol) => {
                let symbol = *symbol;
                let size = symbol.len_utf8();
                Parser(Rc::new(move |state, _| {
                    let text = state.text;
                    if state.index < text.len() && text[state.index..].starts_with(symbol) {
                        state.index += size;
                        true
                    } else {
                        false
                    }
                }))
            }
            /*
            - if-else:
                Or(And(If("left", compare, "right"), if), else)
            - indent:
                And(
                    Set("indent", Value(0)),
                    Loop(And(
                        Push(Symbol('\t')),
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
                    Push(And(
                        Set("$.precedence", Value(Bind::Left => precedence * 2, Bind::Right => precedence * 2 - 1)),
                        node
                    )),
                )
            */
            Node::Precede(precedence, bind, node) => {
                let precedence = *precedence;
                let bind = bind.clone();
                let parser = next(node, context);
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
        }
    }

    let mut context = Context::new();
    let node = context.resolve(node);
    (next(&node, &mut context), context)
}

pub fn parse<'a>(text: &'a str, path: &'a str, node: Node) -> Option<Tree<'a>> {
    let (_, context) = parsers(node);
    let parser = context.reference(&Identifier::Path(path.into()))?;
    let mut state = State {
        index: 0,
        text,
        path: path.into(),
        trees: Vec::new(),
        precedence: 0,
    };

    if parser.0(&mut state, &context) && state.index == state.text.len() {
        state.trees.pop()
    } else {
        None
    }
}

pub fn prefix<N: ToNode>(precedence: usize, node: N) -> Node {
    Node::Precede(precedence, Bind::None, node.node().into())
}

pub fn postfix<N: ToNode>(precedence: usize, bind: Bind, node: N) -> Node {
    Node::Precede(precedence, bind, node.node().into())
}

pub fn precede<L: ToNode, R: ToNode>(prefix: L, postfix: R) -> Node {
    and(prefix, repeat(.., postfix))
}

pub fn symbol(symbol: char) -> Node {
    Node::Symbol(symbol)
}

pub fn range(low: char, high: char) -> Node {
    any((low as u8..=high as u8)
        .into_iter()
        .map(|index| symbol(index as char))
        .collect())
}
