use crate::node::*;
use rand;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::mem;
use std::rc::Rc;

pub struct State {
    pub text: String,
    pub random: ThreadRng,
    pub precedence: usize,
}

pub struct Context {
    pub references: HashMap<usize, Rc<Generate>>,
    pub identifiers: HashMap<String, usize>,
}

pub type Generate = dyn Fn(&mut State, &Context) -> bool;

pub fn generator(node: Node) -> (Rc<Generate>, Context) {
    fn all(node: &Node, context: &mut Context, generates: &mut Vec<Rc<Generate>>) {
        match node {
            Node::And(left, right) => {
                all(left, context, generates);
                all(right, context, generates);
            }
            _ => generates.push(next(node, context)),
        }
    }

    fn any(node: &Node, context: &mut Context, generates: &mut Vec<Rc<Generate>>) {
        match node {
            Node::Or(left, right) => {
                any(left, context, generates);
                any(right, context, generates);
            }
            _ => generates.push(next(node, context)),
        }
    }

    fn next(node: &Node, context: &mut Context) -> Rc<Generate> {
        match node {
            Node::True => Rc::new(|_, _| true),
            Node::False => Rc::new(|_, _| false),
            Node::And(_, _) => {
                let mut generates = Vec::new();
                all(node, context, &mut generates);
                Rc::new(move |state, context| {
                    for generate in generates.iter() {
                        if generate(state, context) {
                            continue;
                        }
                        return false;
                    }
                    true
                })
            }
            Node::Or(_, _) => {
                let mut generates = Vec::new();
                any(node, context, &mut generates);
                Rc::new(move |state, context| {
                    for generate in generates.choose_multiple(&mut state.random, generates.len()) {
                        if generate(state, context) {
                            return true;
                        }
                    }
                    false
                })
            }
            Node::Definition(Identifier::Unique(identifier), node) => {
                let generate = next(node, context);
                context.references.insert(*identifier, generate);
                next(&Node::True, context)
            }
            Node::Reference(Identifier::Unique(identifier)) => {
                match context.references.get(identifier) {
                    Some(generate) => generate.clone(),
                    None => {
                        let identifier = *identifier;
                        Rc::new(move |state, context: &Context| {
                            match context.references.get(&identifier) {
                                Some(generate) => generate(state, context),
                                None => false,
                            }
                        })
                    }
                }
            }
            Node::Spawn(_, node) => next(node, context),
            Node::Symbol(symbol) => {
                let symbol = *symbol;
                Rc::new(move |state, _| {
                    state.text.push(symbol);
                    true
                })
            }
            Node::Precedence(precedence, bind, node) => {
                let precedence = *precedence;
                let bind = bind.clone();
                let parse = next(node, context);
                Rc::new(move |state, context| match bind {
                    Bind::Left if precedence <= state.precedence => false,
                    Bind::Right if precedence < state.precedence => false,
                    _ => {
                        let precedence = mem::replace(&mut state.precedence, precedence);
                        let result = parse(state, context);
                        state.precedence = precedence;
                        result
                    }
                })
            }
            _ => panic!("Invalid node {:?}", node),
        }
    }

    let (node, identifiers) = resolve(node);
    let mut context = Context {
        references: HashMap::new(),
        identifiers,
    };
    (next(&node, &mut context), context)
}

pub fn generate(node: Node) -> Option<String> {
    let (generate, context) = generator(node);
    let mut state = State {
        text: String::new(),
        random: rand::thread_rng(),
        precedence: 0,
    };

    if generate(&mut state, &context) {
        Some(state.text)
    } else {
        None
    }
}
