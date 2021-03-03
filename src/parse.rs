use crate::node::*;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Tree {
    pub kind: String,
    pub value: String,
    pub children: Vec<Tree>,
}

#[derive(Debug, Clone)]
pub struct State {
    pub index: usize,
    pub tree: Tree,
}

pub struct Context {
    pub text: Vec<char>,
    pub references: HashMap<usize, Rc<Parse>>,
    pub identifiers: HashMap<String, usize>,
}

pub type Parse = dyn Fn(&mut State, &Context) -> bool;

/*
    TODO: a parser will receive a '&mut State' that will hold the text pointer, the parse tree, references, precedence, etc.
        - Parse: (&mut State state) -> bool
    TODO: add Node::Birth
    The state can be mutable but must be cloned when facing an 'Or'.
    In the case where 2 branches of an 'Or' succeeds, we know that there is ambiguity in the grammar and the parse
    tree should hold an ambiguity node that points to both results.

    TODO: differentiate between direct and indirect references
        - a direct reference will refer directly to the processor and will not be modifiable
        - an indirect reference will suffer some performance penalty but will be modifiable at runtime

    TODO: collapse pattern 'And('B', And('o', And('b', 'a')))' to a word parser
*/

pub fn parser(node: &Node) -> (Rc<Parse>, Context) {
    fn all(node: &Node, context: &mut Context, parses: &mut Vec<Rc<Parse>>) {
        match node {
            Node::Identity => {}
            Node::And(left, right) => {
                all(left, context, parses);
                all(right, context, parses);
            }
            _ => parses.push(next(node, context)),
        }
    }

    fn any(node: &Node, context: &mut Context, parses: &mut Vec<Rc<Parse>>) -> bool {
        match node {
            Node::Identity => {
                parses.push(Rc::new(|_, _| true));
                false
            }
            Node::Or(left, right) => any(left, context, parses) && any(right, context, parses),
            _ => {
                parses.push(next(node, context));
                true
            }
        }
    }

    fn next(node: &Node, context: &mut Context) -> Rc<Parse> {
        match node {
            Node::Identity => Rc::new(|_, _| true),
            Node::And(_, _) => {
                let mut parses = Vec::new();
                all(node, context, &mut parses);
                if parses.len() == 0 {
                    Rc::new(|_, _| true)
                } else if parses.len() == 1 {
                    parses[0].clone()
                } else {
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
            }
            Node::Or(_, _) => {
                let mut parses = Vec::new();
                any(node, context, &mut parses);
                if parses.len() == 0 {
                    Rc::new(|_, _| false)
                } else if parses.len() == 1 {
                    parses[0].clone()
                } else {
                    Rc::new(move |state, context| {
                        // TODO: if more than 1 parse succeed, we know there is an ambiguity
                        // TODO: run each parse in parallel?
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
            }
            Node::Identifier(Identifier::Unique(identifier), node) => {
                let parse = next(node, context);
                context.references.insert(*identifier, parse.clone());
                parse
            }
            Node::Identifier(Identifier::Name(name), node) => {
                let identifier = unique();
                context.identifiers.insert(name.clone(), identifier);
                let parse = next(node, context);
                context.references.insert(identifier, parse.clone());
                parse
            }
            Node::Reference(Identifier::Unique(identifier)) => {
                match context.references.get(identifier) {
                    Some(parse) => parse.clone(),
                    None => {
                        let identifier = *identifier;
                        Rc::new(move |state, context: &Context| {
                            context
                                .references
                                .get(&identifier)
                                .map(|parse| parse(state, context))
                                .unwrap_or(false)
                        })
                    }
                }
            }
            Node::Reference(Identifier::Name(name)) => match context.identifiers.get(name) {
                Some(identifier) => match context.references.get(identifier) {
                    Some(parse) => parse.clone(),
                    None => {
                        let identifier = *identifier;
                        Rc::new(move |state, context: &Context| {
                            context
                                .references
                                .get(&identifier)
                                .map(|parse| parse(state, context))
                                .unwrap_or(false)
                        })
                    }
                },
                None => {
                    let name = name.clone();
                    Rc::new(move |state, context: &Context| {
                        context
                            .identifiers
                            .get(&name)
                            .and_then(|identifier| context.references.get(identifier))
                            .map(|parse| parse(state, context))
                            .unwrap_or(false)
                    })
                }
            },
            Node::Spawn(kind, node) => {
                let kind = kind.clone();
                let parse = next(node, context);
                Rc::new(move |state, context| {
                    let mut parent = state.tree.clone();
                    let index = state.index;
                    state.tree = Tree {
                        kind: kind.clone(),
                        value: String::new(),
                        children: Vec::new(),
                    };
                    if parse(state, context) {
                        let mut child = state.tree.clone();
                        for i in index..state.index {
                            child.value.push(context.text[i]);
                        }
                        parent.children.push(child);
                        state.tree = parent;
                        true
                    } else {
                        false
                    }
                })
            }
            Node::Character(character) => {
                let character = *character;
                Rc::new(move |state, context| {
                    let text = &context.text;
                    if state.index < text.len() && text[state.index] == character {
                        state.index += 1;
                        true
                    } else {
                        false
                    }
                })
            }
        }
    }
    let mut context = Context {
        text: Vec::new(),
        references: HashMap::new(),
        identifiers: HashMap::new(),
    };
    (next(node, &mut context), context)
}

pub fn parse(text: &str, node: &Node) -> Option<Tree> {
    let tree = Tree {
        kind: String::new(),
        value: text.into(),
        children: Vec::new(),
    };
    let mut state = State { index: 0, tree };
    let (parse, mut context) = parser(node);
    context.text = text.chars().collect();
    if parse(&mut state, &context) {
        Some(state.tree)
    } else {
        None
    }
}

pub fn character(character: char) -> Node {
    Node::Character(character)
}

pub fn word(word: &str) -> Node {
    all(word.chars().map(character).collect())
}
