use crate::usecase::event::AppEvent;
use crate::usecase::normalize::arena::{Arena, Handle};
use crate::usecase::stats::NormalizeStats;
use std::cmp::Ordering;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub async fn global_folder_merge(
    arena: &mut Arena,
    sink: &Option<mpsc::Sender<AppEvent>>,
    stats: &mut NormalizeStats,
) {
    let mut by_name: HashMap<String, Vec<Handle>> = HashMap::new();

    for (h, node) in arena.nodes.iter().enumerate() {
        if node.deleted || node.node_type != "folder" {
            continue;
        }
        if node.depth == 0 {
            continue;
        }
        let Some(name) = node.name.as_ref() else {
            continue;
        };
        let norm = name.trim().to_lowercase();
        by_name.entry(norm).or_default().push(Handle(h));
    }

    let mut keys: Vec<String> = by_name.keys().cloned().collect();
    keys.sort();

    for key in keys {
        let handles = by_name.get(&key).cloned().unwrap_or_default();
        if handles.len() <= 1 {
            continue;
        }

        emit(
            sink,
            AppEvent::FolderMergePlanned {
                normalized_name: key.clone(),
                group_size: handles.len(),
            },
        )
        .await;

        let mut sorted = handles;
        sorted.sort_by(|a, b| compare_folder_instance(arena, *a, *b));

        let winner = sorted[0];
        let losers = sorted[1..].to_vec();

        for loser in losers.iter().copied() {
            merge_folder_into(arena, loser, winner);
            stats.folders_merged += 1;
        }

        let losers_paths: Vec<String> = losers
            .iter()
            .map(|h| arena.nodes[h.0].path.clone())
            .collect();
        emit(
            sink,
            AppEvent::FolderMerged {
                normalized_name: key,
                winner_path: arena.nodes[winner.0].path.clone(),
                losers: losers_paths,
            },
        )
        .await;
    }
}

async fn emit(sink: &Option<mpsc::Sender<AppEvent>>, ev: AppEvent) {
    if let Some(tx) = sink {
        let _ = tx.send(ev).await;
    }
}

fn compare_folder_instance(arena: &Arena, a: Handle, b: Handle) -> Ordering {
    let aa = &arena.nodes[a.0];
    let bb = &arena.nodes[b.0];

    aa.depth
        .cmp(&bb.depth)
        .then_with(|| cmp_date_added(aa, bb))
        .then_with(|| cmp_id(aa, bb))
        .then_with(|| aa.guid.cmp(&bb.guid))
}

fn cmp_date_added(
    a: &crate::usecase::normalize::arena::ArenaNode,
    b: &crate::usecase::normalize::arena::ArenaNode,
) -> Ordering {
    parse_edge_time(a.date_added.as_deref()).cmp(&parse_edge_time(b.date_added.as_deref()))
}

fn parse_edge_time(s: Option<&str>) -> u64 {
    match s {
        Some(v) => v.parse::<u64>().unwrap_or(u64::MAX),
        None => u64::MAX,
    }
}

fn cmp_id(
    a: &crate::usecase::normalize::arena::ArenaNode,
    b: &crate::usecase::normalize::arena::ArenaNode,
) -> Ordering {
    let pa = a.id.as_deref().and_then(|s| s.parse::<u64>().ok());
    let pb = b.id.as_deref().and_then(|s| s.parse::<u64>().ok());

    match (pa, pb) {
        (Some(aa), Some(bb)) => aa.cmp(&bb),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => a.id.cmp(&b.id),
    }
}

fn merge_folder_into(arena: &mut Arena, loser: Handle, winner: Handle) {
    if loser.0 == winner.0 {
        return;
    }
    if arena.nodes[loser.0].deleted {
        return;
    }

    let loser_path = arena.nodes[loser.0].path.clone();
    let loser_name = arena.nodes[loser.0].name.clone();
    let loser_id = arena.nodes[loser.0].id.clone();
    let loser_guid = arena.nodes[loser.0].guid.clone();

    let mut children = std::mem::take(&mut arena.nodes[loser.0].children);
    for ch in children.iter() {
        arena.parent[ch.0] = Some(winner);
    }
    arena.nodes[winner.0].children.append(&mut children);

    if let Some(n) = loser_name {
        arena.nodes[winner.0].merged_names.push(n);
    }
    if let Some(id) = loser_id {
        arena.nodes[winner.0].merged_ids.push(id);
    }
    if let Some(g) = loser_guid {
        arena.nodes[winner.0].merged_guids.push(g);
    }
    arena.nodes[winner.0].merged_paths.push(loser_path);

    if let Some(parent) = arena.parent[loser.0] {
        arena.nodes[parent.0].children.retain(|h| h.0 != loser.0);
    }

    arena.nodes[loser.0].deleted = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecase::normalize::arena::ArenaNode;

    #[test]
    fn parse_edge_time_handles_none_and_invalid() {
        assert_eq!(parse_edge_time(None), u64::MAX);
        assert_eq!(parse_edge_time(Some("not_a_number")), u64::MAX);
        assert_eq!(parse_edge_time(Some("42")), 42);
    }

    #[test]
    fn cmp_id_orders_numeric_before_missing_and_text() {
        let a = ArenaNode {
            id: Some("10".to_string()),
            ..ArenaNode::default()
        };
        let b = ArenaNode {
            id: None,
            ..ArenaNode::default()
        };
        assert_eq!(cmp_id(&a, &b), Ordering::Less);
        assert_eq!(cmp_id(&b, &a), Ordering::Greater);

        let c = ArenaNode {
            id: Some("abc".to_string()),
            ..ArenaNode::default()
        };
        let d = ArenaNode {
            id: Some("def".to_string()),
            ..ArenaNode::default()
        };
        assert_eq!(cmp_id(&c, &d), Ordering::Less);
    }

    #[test]
    fn merge_folder_into_noops_on_self_or_deleted_loser() {
        let mut arena = Arena::default();
        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            name: Some("a".to_string()),
            depth: 1,
            ..ArenaNode::default()
        });
        arena.parent.push(None);

        merge_folder_into(&mut arena, Handle(0), Handle(0));
        assert!(!arena.nodes[0].deleted);

        arena.nodes[0].deleted = true;
        merge_folder_into(&mut arena, Handle(0), Handle(0));
        assert!(arena.nodes[0].deleted);
    }

    #[test]
    fn compare_folder_instance_uses_depth_then_date_then_id_then_guid() {
        let mut arena = Arena::default();

        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            depth: 2,
            date_added: Some("10".to_string()),
            id: Some("7".to_string()),
            guid: Some("a".to_string()),
            ..ArenaNode::default()
        });
        arena.parent.push(None);

        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            depth: 2,
            date_added: Some("10".to_string()),
            id: Some("7".to_string()),
            guid: Some("b".to_string()),
            ..ArenaNode::default()
        });
        arena.parent.push(None);

        assert_eq!(
            compare_folder_instance(&arena, Handle(0), Handle(1)),
            Ordering::Less
        );

        arena.nodes[1].depth = 3;
        assert_eq!(
            compare_folder_instance(&arena, Handle(0), Handle(1)),
            Ordering::Less
        );
    }
}
