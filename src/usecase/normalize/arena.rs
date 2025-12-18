use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Handle(pub usize);

#[derive(Debug, Clone, Default)]
pub struct ArenaNode {
    pub node_type: String,
    pub name: Option<String>,
    pub url: Option<String>,
    pub children: Vec<Handle>,

    pub date_added: Option<String>,
    pub date_modified: Option<String>,
    pub date_last_used: Option<String>,
    pub visit_count: Option<i64>,

    pub guid: Option<String>,
    pub id: Option<String>,
    pub source: Option<String>,
    pub show_icon: Option<bool>,

    pub extra: BTreeMap<String, serde_json::Value>,

    pub root_key: Option<String>,
    pub path: String,
    pub depth: usize,

    pub deleted: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Arena {
    pub nodes: Vec<ArenaNode>,
    pub parent: Vec<Option<Handle>>,
    pub root_container: BTreeMap<String, Handle>,
}

impl Arena {
    pub fn is_root_container(&self, h: Handle) -> bool {
        self.nodes[h.0].depth == 0
    }
}
