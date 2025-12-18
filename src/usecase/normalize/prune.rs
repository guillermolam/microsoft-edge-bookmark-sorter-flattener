use crate::usecase::event::AppEvent;
use crate::usecase::normalize::arena::{Arena, Handle};
use crate::usecase::stats::NormalizeStats;
use tokio::sync::mpsc;

pub async fn prune_empty_folders(
    arena: &mut Arena,
    sink: &Option<mpsc::Sender<AppEvent>>,
    stats: &mut NormalizeStats,
) {
    let postorder = postorder_folders(arena);

    for h in postorder {
        if arena.is_root_container(h) {
            continue;
        }
        if arena.nodes[h.0].deleted || arena.nodes[h.0].node_type != "folder" {
            continue;
        }

        // Collect deleted status first to avoid borrow conflict
        let to_keep: Vec<Handle> = arena.nodes[h.0]
            .children
            .iter()
            .filter(|c| !arena.nodes[c.0].deleted)
            .copied()
            .collect();
        arena.nodes[h.0].children = to_keep;

        let mut has_url = false;
        let mut has_folder = false;
        let mut has_other = false;

        for c in arena.nodes[h.0].children.iter() {
            let t = arena.nodes[c.0].node_type.as_str();
            if t == "url" {
                has_url = true;
            } else if t == "folder" {
                has_folder = true;
            } else {
                has_other = true;
            }
        }

        if !has_url && !has_folder && !has_other {
            if let Some(parent) = arena.parent[h.0] {
                arena.nodes[parent.0].children.retain(|c| c.0 != h.0);
            }
            arena.nodes[h.0].deleted = true;
            stats.folders_pruned += 1;
            emit(
                sink,
                AppEvent::FolderPruned {
                    folder_path: arena.nodes[h.0].path.clone(),
                },
            )
            .await;
        }
    }
}

async fn emit(sink: &Option<mpsc::Sender<AppEvent>>, ev: AppEvent) {
    if let Some(tx) = sink {
        let _ = tx.send(ev).await;
    }
}

fn postorder_folders(arena: &Arena) -> Vec<Handle> {
    let mut out: Vec<Handle> = Vec::new();

    for root in arena.root_container.values().copied() {
        let mut stack: Vec<(Handle, bool)> = vec![(root, false)];
        while let Some((h, expanded)) = stack.pop() {
            if arena.nodes[h.0].deleted {
                continue;
            }
            if arena.nodes[h.0].node_type != "folder" {
                continue;
            }
            if expanded {
                out.push(h);
                continue;
            }
            stack.push((h, true));
            for ch in arena.nodes[h.0].children.iter().rev() {
                stack.push((*ch, false));
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecase::normalize::arena::ArenaNode;

    #[tokio::test]
    async fn does_not_prune_root_container_even_if_empty() {
        let mut arena = Arena::default();
        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            name: Some("root".to_string()),
            depth: 0,
            path: "/".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(None);
        arena.root_container.insert("bookmark_bar".to_string(), Handle(0));

        let mut stats = NormalizeStats::default();
        prune_empty_folders(&mut arena, &None, &mut stats).await;

        assert!(!arena.nodes[0].deleted);
        assert_eq!(stats.folders_pruned, 0);
    }

    #[tokio::test]
    async fn does_not_prune_folder_with_other_child_type() {
        let mut arena = Arena::default();

        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            name: Some("root".to_string()),
            depth: 0,
            path: "/".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(None);
        arena.root_container.insert("bookmark_bar".to_string(), Handle(0));

        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            name: Some("keep".to_string()),
            depth: 1,
            path: "/keep".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(Some(Handle(0)));
        arena.nodes[0].children.push(Handle(1));

        arena.nodes.push(ArenaNode {
            node_type: "other".to_string(),
            name: Some("mystery".to_string()),
            depth: 2,
            path: "/keep/mystery".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(Some(Handle(1)));
        arena.nodes[1].children.push(Handle(2));

        let mut stats = NormalizeStats::default();
        prune_empty_folders(&mut arena, &None, &mut stats).await;

        assert!(!arena.nodes[1].deleted);
        assert_eq!(stats.folders_pruned, 0);
    }

    #[tokio::test]
    async fn does_not_prune_folder_with_child_folder() {
        let mut arena = Arena::default();

        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            name: Some("root".to_string()),
            depth: 0,
            path: "/".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(None);
        arena.root_container.insert("bookmark_bar".to_string(), Handle(0));

        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            name: Some("keep".to_string()),
            depth: 1,
            path: "/keep".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(Some(Handle(0)));
        arena.nodes[0].children.push(Handle(1));

        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            name: Some("child".to_string()),
            depth: 2,
            path: "/keep/child".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(Some(Handle(1)));
        arena.nodes[1].children.push(Handle(2));

        arena.nodes.push(ArenaNode {
            node_type: "url".to_string(),
            name: Some("x".to_string()),
            url: Some("https://example.com".to_string()),
            depth: 3,
            path: "/keep/child/x".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(Some(Handle(2)));
        arena.nodes[2].children.push(Handle(3));

        let mut stats = NormalizeStats::default();
        prune_empty_folders(&mut arena, &None, &mut stats).await;

        assert!(!arena.nodes[1].deleted);
        assert_eq!(stats.folders_pruned, 0);
    }
}
