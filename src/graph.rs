use std::mem;

/*
language Json {
    grammar {
        . = Value;
        ~ = (' ' | '\t' | '\n' | '\r'){..};
        let integer = ~ & '-' & (0 | (1..9 & 0..9{..}));
        let fraction = '.' & 0..9{1..};
        let exponent = ('e'|'E') & ('+' | '-')? & integer;

        syntax Value.Null = "null";
        syntax Value.True = "true";
        syntax Value.False = "false";
        syntax Value.Number = (integer & fraction? & exponent?)!;
        syntax Value.String = ~'"' & ('\u0'..'\u128')! & '"'~;
        syntax Value.Array = "[" & Value{..;","} & "]";
        syntax Value.Object = "{" & (String & ":" & Value){..;","} & "}";
    }

    convert {
        enum Node {
            Null,
            Boolean(value: bool),
            Number(value: f64),
            String(value: String),
            Array(Vec<Node>),
            Object(Vec<(Node, Node)>),
        }

        syntax Value.Null => Node::Null;
        syntax Value.True => Node::Boolean(true);
        syntax Value.False => Node::Boolean(false);
        syntax Value.Number(text: String) => Node::Number(text.parse().unwrap());
        syntax Value.String(text: String) => Node::String(text);
        syntax Value.Array(nodes: Vec<Node>) => Node::Array(nodes);
        syntax Value.Object(keys: Vec<Node>, values: Vec<Node>) => Node::Object(keys.iter().zip(values).collect());
    }
}

language CSharp {
    grammar {
        ~ = (' ' | '\t' | '\n' | '\r'){..};

        syntax Expression.Json = "json" & "{" & Json & "}";
    }

    convert {
        syntax Expression.Json(json: Json::Node) => {
            match json {
                Json::Node::Null => Node::Literal::Null,
                Json::Node::Boolean(value) => Node::Literal::Boolean(value),
                Json::Node::Number(value) =>
            }
        }
    }
}

language Quint {
    grammar {
        . = "language" & Identifier & "{" & Define{..} & "}";
        ~ = (' ' | '\t' | '\n' | '\r'){..};

        let letter = 'a'..'z' | 'A'..'Z';
        let digit = '0'..='9';
        let integer = '0' | ('1'..='9' & digit*);

        syntax Define.Grammar = "grammar" & "{" & Quint.Grammar & "}";
        syntax Define.Convert = "convert" & "{" & Quint.Convert & "}";
        syntax Identifier.Name = (letter & (letter | digit){..})!;
        syntax Identifier.Path = Identifier{..;"."};
    }
}

language Quint.Grammar {
    grammar {
        Quint.Grammar = Define{..};

        syntax Identifier.Root = ".";
        syntax Identifier.Trivia = "~";

        syntax Define.Syntax = "syntax" & Identifier & "=" & Node & ";";
        syntax Define.Let = "let" & Identifier.Name & "=" & Node & ";";
        syntax Define.Root = Identifier & "=" & Node & ";";
        syntax Define.Trivia = "~" & "=" & Node & ";";

        syntax Node.Word = ~ & '"' & ('\u0'..'\u128'){..}! & '"' & ~;
        syntax Node.Symbol = ~ & '\'' & ('\u0'..'\u128'){..}! & '\'' & ~;
        syntax Node.Group = "(" & Node & ")";
        syntax Node.Range = Node.Symbol & ".." & Node.Symbol;
        syntax Node.Refer = Identifier;
        syntax Node.And { priority = 100 } = Node & "&" & Node;
        syntax Node.Or { priority = 100 } = Node & "|" & Node;
        syntax Node.Repeat { priority = 200 } = Node & "{" integer!? & ".." & integer!? & (";" & Node)? "}";
        syntax Node.Store { priority = 300 } = Node & "!";
        syntax Node.Option { priority = 300 } = Node & "?";
    }

    convert {
        struct Path(Vec<String>);
        enum Node {
            True,
            False,
            And(Node, Node),
            Or(Node, Node),
            Define(Path, Node),
            Refer(Path),
            Spawn(Node),
            Store(Node),
            Text(String),
        }

        syntax Identifier.Root => Path(".");
        syntax Identifier.Trivia => Path("~");
        syntax Identifier.Name(name: String) => Path(vec![name]);
        syntax Identifier.Path(names: Vec<String>) = Path(names);

        syntax Define.Syntax(path: Path, node: Node) => Node::Define(path, Spawn(node));
        syntax Define.Let(path: Path, node: Node) => Node::Define(path, node);
        syntax Define.Root(path: Path, node: Node) => Node::Define(path, node);
        syntax Define.Trivia(node: Node) => Node::Define("~", node);

        syntax Node.Word(text: String) => all!(~, Node::Text(word), ~);
        syntax Node.Symbol(text: String) => Node::Text(text);
        syntax Node.Group(node: Node) => node;
        syntax Node.Range(Node::Text(low: String), Node::Text(high: String)) => {
            match (low.chars().first(), high.chars().first()) {
                (Some(low), Some(high)) => any!(low..=high);
                _ => Node::False
            }
        }
        syntax Node.Refer(path: Path) => Node::Refer(path);
        syntax Node.And(left: Node, right: Node) => Node::And(left, right);
        syntax Node.Or(left: Node, right: Node) => Node::Or(left, right);
        syntax Node.Repeat(node: Node, low: Option<usize>, high: Option<usize>, separator: Option<Node>) => {
            // TODO
        }
        syntax Node.Store(node: Node) => Node::Store(node);
        syntax Node.Option(node: Node) => Node::Or(node, Node::True);
    }
}

language Quint.Convert {

}

- the 'Word' syntax wraps its content with trivia while the 'Symbol' does not
- trivia can still be specified manually by using the '~' identifier
- trivia is never stored
- by default, the root syntax of a language will be the 'or' of all the simple path syntax
- the root syntax can be modified by assigning to the language identifier
- reference resolution prioritizes language local definitions and then looks for foreign definitions
- languages can inherit from another language by declaring it as a sub name space
*/

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Node {
    index: usize,
    generation: usize,
}

#[derive(Debug, Clone)]
pub struct Edge {
    link: Link,
    target: Node,
}

#[derive(Debug, Clone)]
pub struct Graph {
    nodes: Vec<(Node, Vec<Edge>)>,
    free: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parser {
    All,
    Any,
    Spawn,
    Store,
    Define(String),
    Refer(String),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    Parent,
    Child,
    Text(String),
    Syntax(String),
    Parser(Parser),
}

impl Graph {
    pub fn new() -> Self {
        Self {
            nodes: vec![(Node::default(), Vec::new())],
            free: vec![Node::default()],
        }
    }

    pub fn count(&self) -> usize {
        self.nodes.len() - self.free.len()
    }

    pub fn create(&mut self) -> Node {
        match self.free.pop() {
            Some(mut node) => {
                node.generation += 1;
                self.nodes[node.index].0 = node;
                node
            }
            None => {
                let node = Node {
                    index: self.nodes.len(),
                    generation: 0,
                };
                self.nodes.push((node, Vec::new()));
                node
            }
        }
    }

    pub fn destroy(&mut self, node: Node) -> bool {
        match self.get_mut(node) {
            Some(pair) => {
                pair.0 = Node::default();
                pair.1.clear();
                self.free.push(node);
                true
            }
            _ => false,
        }
    }

    pub fn edge(&self, node: Node, index: usize) -> Option<&Edge> {
        self.get(node)?.1.get(index)
    }

    pub fn edges(&self, node: Node) -> Option<&Vec<Edge>> {
        Some(&self.get(node)?.1)
    }

    pub fn has(&self, node: Node) -> bool {
        self.get(node).is_some()
    }

    pub fn link(&mut self, source: Node, target: Node, link: Link) -> Option<usize> {
        self.get(target)?;
        let pair = self.get_mut(source)?;
        let index = pair.1.len();
        pair.1.push(Edge { target, link });
        Some(index)
    }

    pub fn unlink(&mut self, node: Node, predicate: impl Fn(Node, &Link) -> bool) -> Option<usize> {
        let pair = self.get_mut(node)?;
        let mut index = 0;
        let mut count = 0;
        while index < pair.1.len() {
            let edge = &pair.1[index];
            if predicate(edge.target, &edge.link) {
                count += 1;
                pair.1.remove(index);
            } else {
                index += 1;
            }
        }
        Some(count)
    }

    pub fn parse(&mut self, text: &str, node: Node) -> Option<Node> {
        #[derive(Clone)]
        struct State<'a> {
            index: usize,
            text: &'a str,
            path: String,
            node: Node,
        }

        fn parser(node: Node, graph: &Graph) -> Option<(&Parser, Node, usize)> {
            let (_, edges) = graph.get(node)?;
            for (index, edge) in edges.iter().enumerate() {
                match &edge.link {
                    Link::Parser(parser) => return Some((parser, edge.target, index)),
                    _ => {}
                };
            }
            None
        }

        fn next(state: &mut State, source: Node, graph: &mut Graph) -> Option<()> {
            match parser(source, graph)? {
                (Parser::All, target, _) => {
                    for child in graph.children(target) {
                        next(state, child, graph)?;
                    }
                    Some(())
                }
                (Parser::Any, target, _) => {
                    for child in graph.children(target) {
                        let mut clone = state.clone();
                        let result = next(&mut clone, child, graph);
                        if result.is_some() {
                            *state = clone;
                            return result;
                        }
                    }
                    None
                }
                (Parser::Define(_), _, _) => Some(()),
                (Parser::Refer(path), mut target, index) => {
                    let path = path.clone();
                    if !graph.has(target) {
                        for node in graph.hierarchy(source) {
                            match parser(node, graph) {
                                Some((Parser::Define(current), node, _)) if current == &path => {
                                    target = node;
                                    graph.get_mut(source)?.1.get_mut(index)?.target = node;
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }

                    let path = mem::replace(&mut state.path, path);
                    next(state, target, graph)?;
                    state.path = path;
                    Some(())
                }
                (Parser::Spawn, target, _) => {
                    let node = graph.target(target, |link| link == &Link::Child)?;
                    let child = graph.create();
                    let parent = mem::replace(&mut state.node, child);
                    let mut body = || {
                        graph.adopt(parent, child)?;
                        graph.link(child, child, Link::Syntax(state.path.clone()))?;
                        next(state, node, graph)?;
                        Some(())
                    };
                    match body() {
                        Some(_) => {
                            state.node = parent;
                            Some(())
                        }
                        None => {
                            for child in graph.descendants(child) {
                                graph.destroy(child);
                            }
                            graph.destroy(child);
                            None
                        }
                    }
                }
                (Parser::Store, target, _) => {
                    let node = graph.target(target, |link| link == &Link::Child)?;
                    let index = state.index;
                    next(state, node, graph)?;
                    let text = state.text.get(index..state.index)?.into();
                    graph.link(state.node, state.node, Link::Text(text))?;
                    Some(())
                }
                (Parser::Text(text), _, _) => {
                    let slice = state.text.get(state.index..state.index + text.len())?;
                    if slice == text {
                        state.index += text.len();
                        Some(())
                    } else {
                        None
                    }
                }
            }
        }

        let root = self.create();
        let mut state = State {
            index: 0,
            text,
            path: "root".into(),
            node: root,
        };
        next(&mut state, node, self)?;
        if state.index == state.text.len() {
            Some(root)
        } else {
            self.destroy(root);
            None
        }
    }

    pub fn target(&self, node: Node, filter: impl Fn(&Link) -> bool) -> Option<Node> {
        let (_, edges) = self.get(node)?;
        for edge in edges {
            if filter(&edge.link) {
                return Some(edge.target);
            }
        }
        None
    }

    pub fn targets<'a>(
        &'a self,
        node: Node,
        filter: impl Fn(&Link) -> bool + 'a,
    ) -> Option<impl Iterator<Item = Node> + 'a> {
        let (_, edges) = self.get(node)?;
        Some(edges.iter().filter_map(move |edge| {
            if filter(&edge.link) {
                Some(edge.target)
            } else {
                None
            }
        }))
    }

    fn get(&self, node: Node) -> Option<&(Node, Vec<Edge>)> {
        self.nodes.get(node.index).filter(|pair| pair.0 == node)
    }

    fn get_mut(&mut self, node: Node) -> Option<&mut (Node, Vec<Edge>)> {
        self.nodes.get_mut(node.index).filter(|pair| pair.0 == node)
    }
}
