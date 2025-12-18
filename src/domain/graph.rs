#[derive(Debug, Clone, Default)]
pub struct Graph {
    pub edges: Vec<Vec<usize>>,
}

impl Graph {
    pub fn new(node_count: usize) -> Self {
        Self {
            edges: vec![Vec::new(); node_count],
        }
    }

    pub fn node_count(&self) -> usize {
        self.edges.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.iter().map(|v| v.len()).sum()
    }
}

#[derive(Debug, Clone)]
pub struct SccResult {
    pub component_of: Vec<usize>,
    pub components: Vec<Vec<usize>>,
    pub cyclic_component: Vec<bool>,
}
