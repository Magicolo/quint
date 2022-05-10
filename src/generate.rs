use crate::node::*;
use rand;
use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::rc::Rc;
use Identifier::*;
use Node::*;

#[derive(Clone)]
pub struct Generator {
    root: Generate,
    references: Vec<Generate>,
    node_indices: HashMap<Identifier, usize>,
    value_indices: HashMap<Identifier, usize>,
}

struct State<'a> {
    pub text: String,
    pub random: ThreadRng,
    pub references: &'a Vec<Generate>,
    pub precedence: usize,
}

type Generate = Rc<dyn Fn(&mut State) -> bool>;

impl Generator {
    pub fn generate(&self) -> Option<String> {
        let mut state = State {
            text: String::new(),
            random: rand::thread_rng(),
            references: &self.references,
            precedence: 0,
        };

        if (self.root)(&mut state) {
            Some(state.text)
        } else {
            None
        }
    }
}

impl From<Node> for Generator {
    fn from(node: Node) -> Generator {
        fn next(node: &Node, generators: &Vec<Option<Generate>>) -> Generate {
            match node {
                True => Rc::new(|_| true),
                False => Rc::new(|_| false),
                And(_, _) => {
                    let nodes = node.flatten();
                    let generators: Vec<_> =
                        nodes.iter().map(|node| next(node, generators)).collect();
                    Rc::new(move |state| {
                        for generator in &generators {
                            if generator(state) {
                                continue;
                            }
                            return false;
                        }
                        true
                    })
                }
                Or(_, _) => {
                    let nodes = node.flatten();
                    let generators: Vec<_> =
                        nodes.iter().map(|node| next(node, generators)).collect();
                    Rc::new(move |state| {
                        for generator in
                            generators.choose_multiple(&mut state.random, generators.len())
                        {
                            if generator(state) {
                                return true;
                            }
                        }
                        false
                    })
                }
                Refer(Index(index)) => {
                    let index = *index;
                    match &generators[index] {
                        Some(generator) => generator.clone(),
                        None => Rc::new(move |state| state.references[index].clone()(state)),
                    }
                }
                Spawn(_) => next(&True, generators),
                Depth(_) => next(&True, generators),
                Store(_, _) => next(&True, generators),
                Precede(_, _, _) => next(&True, generators),
                Symbol(symbol) => {
                    let symbol = *symbol;
                    Rc::new(move |state| {
                        state.text.push(symbol);
                        true
                    })
                }
                Text(text) => {
                    let text = text.clone();
                    Rc::new(move |state| {
                        state.text.push_str(text.as_str());
                        true
                    })
                }
                Switch(cases) => {
                    let mut nodes = Vec::new();
                    for case in cases {
                        nodes.push(and(case.0, case.1.clone()));
                    }
                    next(&any(nodes), generators)
                }
                node => panic!("Invalid node '{}'.", node),
            }
        }

        let (node, nodes, node_indices, value_indices) = node.resolve();
        let mut generators = vec![None; nodes.len()];
        for i in 0..nodes.len() {
            generators[i] = Some(next(&nodes[i], &generators));
        }
        let root = next(&node, &generators);
        let references = generators
            .drain(..)
            .map(|generator| generator.unwrap())
            .collect();
        Generator {
            root,
            references,
            node_indices,
            value_indices,
        }
    }
}
