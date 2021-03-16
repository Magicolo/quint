use crate::node::*;
use rand;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use std::mem;
use std::rc::Rc;
use Node::*;

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
            True => Generator(Rc::new(|_, _| true)),
            False => Generator(Rc::new(|_, _| false)),
            And(_, _) => {
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
            Or(_, _) => {
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
            Define(identifier, node) => {
                let generator = next(node, context);
                context.refer(identifier, generator);
                next(&True, context)
            }
            Refer(identifier) => {
                let identifier = context.identify(identifier);
                Generator(Rc::new(move |state, context| {
                    match context.references.get(&identifier) {
                        Some(generator) => generator.0(state, context),
                        None => false,
                    }
                }))
            }
            Spawn(_) => next(&True, context),
            Depth(_) => next(&True, context),
            Store(_, _) => next(&True, context),
            Precede(_, _, _) => next(&True, context),
            Symbol(symbol) => {
                let symbol = *symbol;
                Generator(Rc::new(move |state, _| {
                    state.text.push(symbol);
                    true
                }))
            }
            Text(text) => {
                let text = text.clone();
                Generator(Rc::new(move |state, _| {
                    state.text.push_str(text.as_str());
                    true
                }))
            }
            Switch(cases) => {
                let mut nodes = Vec::new();
                for case in cases {
                    nodes.push(and(case.0, case.1.clone()));
                }
                next(&any(nodes), context)
            }
            node => panic!("Invalid node '{}'.", node),
        }
    }

    let mut context = Context::new();
    let node = context.resolve(node);
    (next(&node, &mut context), context)
}

pub fn generate(node: Node) -> Option<String> {
    let (generator, context) = generator(node);
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
