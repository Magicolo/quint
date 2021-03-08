use crate::graph::*;

impl Graph {
    pub fn root(&self, node: Node) -> Node {
        self.parent(node)
            .and_then(|node| self.parent(node))
            .unwrap_or(node)
    }

    pub fn hierarchy(&self, node: Node) -> Vec<Node> {
        let root = self.root(node);
        let mut nodes = self.descendants(root);
        nodes.push(root);
        nodes
    }

    pub fn parent(&self, node: Node) -> Option<Node> {
        self.target(node, |link| link == &Link::Parent)
    }

    pub fn children(&self, node: Node) -> Vec<Node> {
        match self.targets(node, |link| link == &Link::Child) {
            Some(children) => children.collect(),
            None => Vec::new(),
        }
    }

    pub fn descendants(&self, node: Node) -> Vec<Node> {
        fn descend(node: Node, graph: &Graph, nodes: &mut Vec<Node>) {
            match graph.targets(node, |link| link == &Link::Child) {
                Some(children) => {
                    for child in children {
                        descend(child, graph, nodes);
                        nodes.push(child);
                    }
                }
                None => {}
            };
        }
        let mut nodes = Vec::new();
        descend(node, self, &mut nodes);
        nodes
    }

    pub fn adopt(&mut self, parent: Node, child: Node) -> Option<(usize, usize)> {
        Some((
            self.link(parent, child, Link::Child)?,
            self.link(child, parent, Link::Parent)?,
        ))
    }
}
