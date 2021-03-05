use crate::node::*;
use rand;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use std::mem;
use std::rc::Rc;

pub struct State {
    pub text: String,
    pub random: ThreadRng,
    pub precedence: usize,
}

#[derive(Clone)]
pub struct Generator(Rc<Generate>);
pub type Generate = dyn Fn(&mut State, &Context<Generator>) -> bool;

pub fn generator(node: Node) -> (Generator, Context<Generator>) {
    fn next(node: &Node, context: &mut Context<Generator>) -> Generator {
        match node {
            Node::True => Generator(Rc::new(|_, _| true)),
            Node::False => Generator(Rc::new(|_, _| false)),
            Node::And(_, _) => {
                let nodes = node.flatten();
                let generators: Vec<_> = nodes.iter().map(|node| next(node, context)).collect();
                Generator(Rc::new(move |state, context| {
                    for generator in &generators {
                        if generator.0(state, context) {
                            continue;
                        }
                        return false;
                    }
                    true
                }))
            }
            Node::Or(_, _) => {
                let nodes = node.flatten();
                let generators: Vec<_> = nodes.iter().map(|node| next(node, context)).collect();
                Generator(Rc::new(move |state, context| {
                    for generator in generators.choose_multiple(&mut state.random, generators.len())
                    {
                        if generator.0(state, context) {
                            return true;
                        }
                    }
                    false
                }))
            }
            Node::Define(identifier, node) => {
                let generator = next(node, context);
                context.refer(identifier, generator);
                next(&Node::True, context)
            }
            Node::Refer(identifier) => {
                let identifier = context.identify(identifier);
                Generator(Rc::new(move |state, context| {
                    match context.references.get(&identifier) {
                        Some(generator) => generator.0(state, context),
                        None => false,
                    }
                }))
            }
            Node::Spawn(node) => next(node, context),
            Node::Symbol(symbol) => {
                let symbol = *symbol;
                Generator(Rc::new(move |state, _| {
                    state.text.push(symbol);
                    true
                }))
            }
            Node::Precede(precedence, bind, node) => {
                let precedence = *precedence;
                let bind = bind.clone();
                let generator = next(node, context);
                Generator(Rc::new(move |state, context| match bind {
                    Bind::Left if precedence <= state.precedence => false,
                    Bind::Right if precedence < state.precedence => false,
                    _ => {
                        let precedence = mem::replace(&mut state.precedence, precedence);
                        let result = generator.0(state, context);
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

pub fn generate(node: Node, path: &str) -> Option<String> {
    let (_, context) = generator(node);
    let generator = context.reference(&Identifier::Path(path.into()))?;
    let mut state = State {
        text: String::new(),
        random: rand::thread_rng(),
        precedence: 0,
    };

    if generator.0(&mut state, &context) {
        Some(state.text)
    } else {
        None
    }
}
