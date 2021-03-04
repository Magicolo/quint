use crate::node::*;
use std::collections::HashMap;
use std::mem;
use std::ops::{Range, RangeInclusive};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Tree<'a> {
    pub kind: String,
    pub value: &'a str,
    pub children: Vec<Tree<'a>>,
}

#[derive(Debug, Clone)]
pub struct State<'a> {
    pub index: usize,
    pub tree: Tree<'a>,
}

pub struct Context<'a> {
    pub text: &'a str,
    pub references: HashMap<usize, Rc<Parse<'a>>>,
    pub identifiers: HashMap<String, usize>,
}

impl AsNode for char {
    fn as_node(self) -> Node {
        Node::Symbol(self)
    }
}

impl AsNode for &str {
    fn as_node(self) -> Node {
        all(self.chars().map(symbol).collect())
    }
}

impl AsNode for Range<char> {
    fn as_node(self) -> Node {
        range(self.start, self.end)
    }
}

impl AsNode for RangeInclusive<char> {
    fn as_node(self) -> Node {
        range(*self.start(), *self.end())
    }
}

pub type Parse<'a> = dyn Fn(&mut State<'a>, &Context<'a>) -> bool + 'a;

/*
    The state can be mutable but must be cloned when facing an 'Or'.
    In the case where 2 branches of an 'Or' succeeds, we know that there is ambiguity in the grammar and the parse
    tree should hold an ambiguity node that points to both results.

    TODO: differentiate between direct and indirect references
        - a direct reference will refer directly to the processor and will not be modifiable
        - an indirect reference will suffer some performance penalty but will be modifiable at runtime

    TODO: try to remove 'String.clone()' especially in the spawn logic
    TODO: collapse pattern ''B' & 'o' & 'b' & 'a'' to a word parser '"Boba"'
    TODO: collapse pattern '('A' & A) | ('B' & B) | ('C' & C)' to a map parser { 'A': A, 'B': B, 'C': C }
    TODO: add a state node
    TODO: operator precedence parser

    TODO: retain ambiguities when both branches of an 'Or' succeeds?
    TODO: run each branch of an 'Or' in parallel?
*/

pub fn parser<'a>(node: &Node) -> (Rc<Parse<'a>>, Context<'a>) {
    fn all<'a>(node: &Node, context: &mut Context<'a>, parses: &mut Vec<Rc<Parse<'a>>>) {
        match node {
            Node::And(left, right) => {
                all(left, context, parses);
                all(right, context, parses);
            }
            _ => parses.push(next(node, context)),
        }
    }

    fn any<'a>(node: &Node, context: &mut Context<'a>, parses: &mut Vec<Rc<Parse<'a>>>) {
        match node {
            Node::Or(left, right) => {
                any(left, context, parses);
                any(right, context, parses);
            }
            _ => parses.push(next(node, context)),
        }
    }

    fn next<'a>(node: &Node, context: &mut Context<'a>) -> Rc<Parse<'a>> {
        match node {
            Node::True => Rc::new(|_, _| true),
            Node::False => Rc::new(|_, _| false),
            Node::And(_, _) => {
                let mut parses = Vec::new();
                all(node, context, &mut parses);
                Rc::new(move |state, context| {
                    for parse in &parses {
                        if parse(state, context) {
                            continue;
                        }
                        return false;
                    }
                    true
                })
            }
            Node::Or(_, _) => {
                let mut parses = Vec::new();
                any(node, context, &mut parses);
                Rc::new(move |state, context| {
                    for parse in &parses {
                        let mut local = state.clone();
                        if parse(&mut local, context) {
                            *state = local;
                            return true;
                        }
                    }
                    false
                })
            }
            Node::Definition(Identifier::Unique(identifier), node) => {
                let parse = next(node, context);
                context.references.insert(*identifier, parse);
                next(&Node::True, context)
            }
            Node::Reference(Identifier::Unique(identifier)) => {
                match context.references.get(identifier) {
                    Some(parse) => parse.clone(),
                    None => {
                        let identifier = *identifier;
                        Rc::new(move |state, context: &Context| {
                            match context.references.get(&identifier) {
                                Some(parse) => parse(state, context),
                                None => false,
                            }
                        })
                    }
                }
            }
            Node::Spawn(kind, node) => {
                let kind = kind.clone();
                let parse = next(node, context);
                Rc::new(move |state, context| {
                    let index = state.index;
                    let parent = mem::replace(
                        &mut state.tree,
                        Tree {
                            kind: kind.clone(),
                            value: "",
                            children: Vec::new(),
                        },
                    );
                    if parse(state, context) {
                        let mut child = mem::replace(&mut state.tree, parent);
                        child.value = &context.text[index..state.index];
                        state.tree.children.push(child);
                        true
                    } else {
                        false
                    }
                })
            }
            Node::Symbol(symbol) => {
                let symbol = *symbol;
                let size = symbol.len_utf8();
                Rc::new(move |state, context| {
                    let text = context.text;
                    if state.index < text.len() && text[state.index..].starts_with(symbol) {
                        state.index += size;
                        true
                    } else {
                        false
                    }
                })
            }
            _ => panic!("Invalid node {:?}.", node),
        }
    }

    let mut context = Context {
        text: "",
        references: HashMap::new(),
        identifiers: HashMap::new(),
    };
    let node = node
        .clone()
        .descend(|node| match node {
            Node::And(left, right) if *left == Node::True => *right,
            Node::And(left, right) if *right == Node::True => *left,
            Node::And(left, _) if *left == Node::False => Node::False,
            Node::Or(left, right) if *left == Node::False => *right,
            Node::Or(left, _) if *left == Node::True => Node::True,
            _ => node,
        })
        .descend(|node| match node {
            Node::Definition(Identifier::Name(name), node) => {
                let identifier = Node::unique();
                context.identifiers.insert(name.clone(), identifier);
                Node::Definition(Identifier::Unique(identifier), node)
            }
            _ => node,
        })
        .descend(|node| match node {
            Node::Reference(Identifier::Name(name)) => {
                Node::Reference(Identifier::Unique(match context.identifiers.get(&name) {
                    Some(identifier) => *identifier,
                    None => Node::unique(),
                }))
            }
            _ => node,
        });

    (next(&node, &mut context), context)
}

pub fn parse<'a>(text: &'a str, node: &Node) -> Option<Tree<'a>> {
    let tree = Tree {
        kind: "root".into(),
        value: text,
        children: Vec::new(),
    };
    let mut state = State { index: 0, tree };
    let (parse, mut context) = parser(node);
    context.text = text;
    if parse(&mut state, &context) && state.index == context.text.len() {
        Some(state.tree)
    } else {
        None
    }
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
