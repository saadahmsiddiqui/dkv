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
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_nodes_to_cluster() {
        let mut cluster = Cluster::new();

        cluster.add_node(Node::new("node-a", "127.0.0.1:6379"));
        cluster.add_node(Node::new("node-b", "127.0.0.1:6380"));
        cluster.add_node(Node::new("node-c", "127.0.0.1:6381"));

        assert_eq!(cluster.len(), 3);
        assert!(!cluster.is_empty());
        assert_eq!(cluster.nodes()[0].id, "node-a");
        assert_eq!(cluster.nodes()[1].address, "127.0.0.1:6380");
    }
}
