use crate::domain::graph::Graph;
use crate::usecase::normalize::arena::Arena;
use serde_json::json;
use std::collections::HashMap;

pub fn build_identity_graph(arena: &Arena) -> (Graph, serde_json::Value) {
    let mut ids: Vec<String> = Vec::new();
    let mut handle_to_id: Vec<Option<String>> = vec![None; arena.nodes.len()];

    for (h, node) in arena.nodes.iter().enumerate() {
        if node.node_type != "folder" {
            continue;
        }
        let key = node
            .guid
            .clone()
            .or_else(|| node.id.clone())
            .unwrap_or_else(|| format!("path:{}", node.path));
        handle_to_id[h] = Some(key);
    }

    for key in handle_to_id.iter().flatten() {
        ids.push(key.clone());
    }
    ids.sort();
    ids.dedup();

    let mut id_index: HashMap<String, usize> = HashMap::new();
    for (i, k) in ids.iter().enumerate() {
        id_index.insert(k.clone(), i);
    }

    let mut g = Graph::new(ids.len());

    for (parent_h, parent_node) in arena.nodes.iter().enumerate() {
        if parent_node.node_type != "folder" {
            continue;
        }
        let Some(parent_key) = handle_to_id[parent_h].as_ref() else {
            continue;
        };
        let p = id_index[parent_key];

        for child in parent_node.children.iter() {
            let child_node = &arena.nodes[child.0];
            if child_node.node_type != "folder" {
                continue;
            }
            let Some(child_key) = handle_to_id[child.0].as_ref() else {
                continue;
            };
            let c = id_index[child_key];
            g.edges[p].push(c);
        }
    }

    for outs in g.edges.iter_mut() {
        outs.sort_unstable();
        outs.dedup();
    }

    (g, json!({"identity_nodes": ids.len()}))
}
