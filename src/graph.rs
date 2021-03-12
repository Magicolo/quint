use std::mem;

/*
language Json {
    parse {
        . = Value;
        ~ = (' ' | '\t' | '\n' | '\r'){..};
        let integer = ~ & '-' & (0 | (1..9 & 0..9{..}));
        let fraction = '.' & 0..9{1..};
        let exponent = ('e'|'E') & ('+' | '-')? & integer;

        Value.Null = "null";
        Value.True = "true";
        Value.False = "false";
        Value.Number = (integer & fraction? & exponent?)!;
        Value.String = ~'"' & ('\u0'..'\u128')! & '"'~;
        Value.Array = "[" & Value{..;","} & "]";
        Value.Object = "{" & (String & ":" & Value){..;","} & "}";
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

        Value.Null => Node::Null;
        Value.True => Node::Boolean(true);
        Value.False => Node::Boolean(false);
        Value.Number(text: String) => Node::Number(text.parse().unwrap());
        Value.String(text: String) => Node::String(text);
        Value.Array(nodes: Vec<Node>) => Node::Array(nodes);
        Value.Object(keys: Vec<Node>, values: Vec<Node>) => Node::Object(keys.iter().zip(values).collect());
    }
}

language CSharp {
    parse {
        . = (Directive | Declaration){..};
        ~ = (space | line | Comment){..};

        let space = ' ' | '\t' | '\0';
        let line = '\n' | '\r' | '\0';
        let integer = '0' | '1'..='9' & '0'..='9'{..};
        let fraction = '.' & '0'..='9'{1..};
        let preUnary(operator) = operator & Expression;
        let postUnary(operator) = Expression & operator;
        let binary(operator) = Expression & operator & Expression;
        let accessor(name) = name & (("=>" & Expression) | Statement.Block);
        let kind = "class" | "interface" | "struct";

        Comment.Line = block('//', true!, line);
        Comment.Block = block('/' & '*', true, '*' * '/');

        Name.Global = "global";
        Name.Name = ~ & (('a'..='z' | 'A'..='Z') & ('a'..='z' | 'A'..='Z' | '0'..='9'){..})! & ~;
        Name.Qualified = Name.Name & "." & Name;

        Directive.Using = "using" & Name & ";";
        Directive.Extern = "extern" & "alias" & Name & ";";

        Accessor.Get = accessor("get");
        Accessor.Set = accessor("set");
        Accessor.Add = accessor("add");
        Accessor.Remove = accessor("remove");

        Literal.Null = "null";
        Literal.True = "true";
        Literal.False = "false";
        Literal.Integer = integer! & ('u' | 'l' | 'ul')!;
        Literal.Rational = integer!? & '.' & integer! & ('f' | 'd' | 'm')!;
        Literal.String = block(~ & '"', '\"'?!, '"' & ~);
        Literal.Symbol = block(~ & '\'', '\"'?!, '\'' & ~);

        Declaration.Namespace = "namespace" & block("{", (Directive | Declaration){..}, "}");
        Declaration.Type = kind! & block("{", Declaration{..}, "}");

        Statement.Empty = ";";
        Statement.Block = "{" & Statement{..} & "}";
        Statement.If = "if" & "(" & Expression & ")" & Statement & ("else" & Statement)?;
        Statement.Return = "return" & Expression & ";";
        Statement.Expression = Expression & ";";
        Statement.Continue = "continue" & ";";
        Statement.Break = "break" & ";";
        Statement.Yield = "yield" & (Statement.Return | Statement.Break) & ";";

        Expression.Literal = Literal;
        Expression.Group = "(" & Expression & ")";
        Expression.Name = Name;

        Expression.LogicNot = preUnary("!");
        Expression.BitNot = preUnary("~");
        Expression.PreIncrement = preUnary("++");
        Expression.PreDecrement = preUnary("--");
        Expression.PostIncrement = postUnary("++");
        Expression.PostDecrement = postUnary("--");

        Expression.Invoke = Expression & block("(", Expression{..;","}, ")");
        Expression.Index = Expression & block("[", Expression{..;","}, "]");
        Expression.Access = Expression & "." & Identifier.Name;
        Expression.Dereference = Expression & "->" & Identifier.Name;

        Expression.LogicAnd = binary("&&");
        Expression.LogicOr = binary("||");
        Expression.BitAnd = binary("&");
        Expression.BitOr = binary("|");
        Expression.BitXor = binary("^");
        Expression.Add = binary("+");
        Expression.Subtract = binary("-");
        Expression.Multiply = binary("*");
        Expression.Divide = binary("/");
        Expression.ShiftLeft = binary("<<");
        Expression.ShiftRight = binary(">>");
        Expression.Equal = binary("==");
        Expression.NotEqual = binary("!=");
        Expression.Greater = binary(">");
        Expression.GreaterEqual = binary(">=");
        Expression.Lesser = binary("<");
        Expression.LesserEqual = binary("<=");
        Expression.Is = binary("is");
        Expression.As = binary("as");
        Expression.Assign { bind: right } = binary("=");
        Expression.AddAssign { bind: right } = binary("+=");
        Expression.SubtractAssign { bind: right } = binary("-=");
        Expression.MultiplyAssign { bind: right } = binary("*=");
        Expression.DivideAssign { bind: right } = binary("/=");
        Expression.If { bind: right } = Expression & "?" & Expression & ":" & Expression;

        Expression.Json = "json" & "{" & Json & "}";
    }

    convert {
        Expression.Json(Json::Node::Null) => Literal::Null;
        Expression.Json(Json::Node::Boolean(value: bool)) => Literal::Boolean(value);
        Expression.Json(Json::Node::Number(value: f64)) => Literal::Rational(value);
    }
}

language Quint {
    parse {
        . = "language" & Identifier & "{" & Define{..} & "}" |> check |> interpret;
        ~ = (' ' | '\t' | '\n' | '\r'){..};

        let letter = 'a'..'z' | 'A'..'Z';
        let digit = '0'..='9';
        let integer = '0' | ('1'..='9' & digit*);

        Define.Grammar = "parse" & "{" & Quint.Grammar & "}";
        Define.Convert = "convert" & "{" & Quint.Convert & "}";
        Define.Check = "check" & "{" & Quint.Check & "}";
        Define.Interpret = "interpret" & "{" & Quint.Interpret & "}";
        Define.Generate = "generate" & Identifier & "{" & Quint.Generate & "}";

        Identifier.Name = (letter & (letter | digit){..})!;
        Identifier.Path = Identifier{..;"."};
    }
}

language Quint.Grammar {
    parse {
        Quint.Grammar = Define{..};

        Identifier.Root = ".";
        Identifier.Trivia = "~";

        Define.Syntax = Identifier & "=" & Node & ";";
        Define.Let = "let" & Identifier.Name & "=" & Node & ";";
        Define.Trivia = "~" & "=" & Node & ";";

        Node.Word = ~ & '"' & ('\u0'..'\u128'){..}! & '"' & ~;
        Node.Symbol = ~ & '\'' & ('\u0'..'\u128'){..}! & '\'' & ~;
        Node.Group = "(" & Node & ")";
        Node.Range = Node.Symbol & ".." & Node.Symbol;
        Node.Refer = Identifier;
        Node.And { priority = 100 } = Node & "&" & Node;
        Node.Or { priority = 100 } = Node & "|" & Node;
        Node.Repeat { priority = 200 } = Node & "{" integer!? & ".." & integer!? & (";" & Node)? "}";
        Node.Store { priority = 300 } = Node & "!";
        Node.Option { priority = 300 } = Node & "?";
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

        Identifier.Root => Path(".");
        Identifier.Trivia => Path("~");
        Identifier.Name(name: String) => Path(vec![name]);
        Identifier.Path(names: Vec<String>) = Path(names);

        Define.Syntax(path: Path, node: Node) => Node::Define(path, Spawn(node));
        Define.Let(path: Path, node: Node) => Node::Define(path, node);
        Define.Trivia(node: Node) => Node::Define("~", node);

        Node.Word(text: String) => all!(~, Node::Text(word), ~);
        Node.Symbol(text: String) => Node::Text(text);
        Node.Group(node: Node) => node;
        Node.Range(Node::Text(low: String), Node::Text(high: String)) => {
            match (low.chars().first(), high.chars().first()) {
                (Some(low), Some(high)) => any!(low..=high);
                _ => Node::False
            }
        }
        Node.Refer(path: Path) => Node::Refer(path);
        Node.And(left: Node, right: Node) => Node::And(left, right);
        Node.Or(left: Node, right: Node) => Node::Or(left, right);
        Node.Repeat(node: Node, low: Option<usize>, high: Option<usize>, separator: Option<Node>) => {
            // TODO
        }
        Node.Store(node: Node) => Node::Store(node);
        Node.Option(node: Node) => Node::Or(node, Node::True);
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
