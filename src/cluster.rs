#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub address: String,
}

impl Node {
    pub fn new(id: impl Into<String>, address: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            address: address.into(),
        }
    }
}

#[derive(Clone)]
pub struct Cluster {
    nodes: Vec<Node>,
}

impl Cluster {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn default() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.push(node);
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

impl Default for Cluster {
    fn default() -> Self {
        Self::default()
    }
}