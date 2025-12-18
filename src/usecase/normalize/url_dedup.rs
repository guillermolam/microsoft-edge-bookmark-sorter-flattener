use crate::domain::traits::UrlCanonicalizer;
use crate::usecase::event::AppEvent;
use crate::usecase::normalize::arena::{Arena, Handle};
use crate::usecase::stats::NormalizeStats;
use serde_json::json;
use std::cmp::Ordering;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub async fn per_folder_url_dedup(
    arena: &mut Arena,
    canonicalizer: &dyn UrlCanonicalizer,
    sink: &Option<mpsc::Sender<AppEvent>>,
    stats: &mut NormalizeStats,
) {
    for folder_h in 0..arena.nodes.len() {
        if arena.nodes[folder_h].deleted {
            continue;
        }
        if arena.nodes[folder_h].node_type != "folder" {
            continue;
        }

        let mut best: HashMap<String, Handle> = HashMap::new();
        let mut removed_by_url: HashMap<String, Vec<Handle>> = HashMap::new();

        // Snapshot children list to avoid borrow fights.
        let children = arena.nodes[folder_h].children.clone();
        for ch in children {
            if arena.nodes[ch.0].deleted {
                continue;
            }
            if arena.nodes[ch.0].node_type != "url" {
                continue;
            }
            let Some(url) = arena.nodes[ch.0].url.clone() else {
                continue;
            };

            let canon = canonicalizer.canonicalize(&url);
            match best.get(&canon).copied() {
                None => {
                    best.insert(canon, ch);
                }
                Some(existing) => {
                    let winner = pick_url_winner(&arena.nodes[existing.0], &arena.nodes[ch.0]);
                    match winner {
                        UrlWinner::KeepExisting => {
                            removed_by_url.entry(canon).or_default().push(ch);
                        }
                        UrlWinner::KeepNew => {
                            removed_by_url.entry(canon.clone()).or_default().push(existing);
                            best.insert(canon, ch);
                        }
                    }
                }
            }
        }

        if removed_by_url.is_empty() {
            continue;
        }

        for (canon, removed) in removed_by_url {
            let Some(&winner) = best.get(&canon) else { continue };

            for rm in removed.iter() {
                // Clone node data first to avoid borrow conflict
                let rm_id = arena.nodes[rm.0].id.clone();
                let rm_guid = arena.nodes[rm.0].guid.clone();
                let rm_name = arena.nodes[rm.0].name.clone();
                let rm_url = arena.nodes[rm.0].url.clone();
                let rm_path = arena.nodes[rm.0].path.clone();
                
                arena.nodes[winner.0].merged_from.push(json!({
                    "id": rm_id,
                    "guid": rm_guid,
                    "name": rm_name,
                    "url": rm_url,
                    "path": rm_path,
                }));
                arena.nodes[rm.0].deleted = true;
            }

            // Collect non-deleted children to avoid borrow conflict
            let to_keep: Vec<Handle> = arena.nodes[folder_h]
                .children
                .iter()
                .filter(|h| !arena.nodes[h.0].deleted)
                .copied()
                .collect();
            arena.nodes[folder_h].children = to_keep;

            stats.urls_deduped += removed.len();

            emit(
                sink,
                AppEvent::UrlDeduped {
                    folder_path: arena.nodes[folder_h].path.clone(),
                    canonical_url: canon,
                    removed: removed.len(),
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

enum UrlWinner {
    KeepExisting,
    KeepNew,
}

fn pick_url_winner(existing: &crate::usecase::normalize::arena::ArenaNode, new: &crate::usecase::normalize::arena::ArenaNode) -> UrlWinner {
    // Winner selection:
    // 1) highest visit_count
    // 2) latest date_last_used
    // 3) earliest date_added
    // 4) smallest id
    let ex = url_rank(existing);
    let nw = url_rank(new);
    match nw.cmp(&ex) {
        Ordering::Greater => UrlWinner::KeepNew,
        _ => UrlWinner::KeepExisting,
    }
}

fn url_rank(n: &crate::usecase::normalize::arena::ArenaNode) -> (i64, u64, std::cmp::Reverse<u64>, std::cmp::Reverse<IdKey>) {
    let visit = n.visit_count.unwrap_or(0);
    let last_used = parse_edge_time(n.date_last_used.as_deref());
    let date_added = parse_edge_time(n.date_added.as_deref());
    (visit, last_used, std::cmp::Reverse(date_added), std::cmp::Reverse(id_key(n.id.as_deref())))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum IdKey {
    Parsed(u64),
    Text(String),
    Missing,
}

fn id_key(id: Option<&str>) -> IdKey {
    let Some(id) = id else {
        return IdKey::Missing;
    };
    if let Ok(v) = id.parse::<u64>() {
        return IdKey::Parsed(v);
    }
    IdKey::Text(id.to_string())
}

fn parse_edge_time(s: Option<&str>) -> u64 {
    match s {
        Some(v) => v.parse::<u64>().unwrap_or(0),
        None => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::url_canonicalizer::DefaultUrlCanonicalizer;
    use crate::usecase::normalize::arena::ArenaNode;

    #[test]
    fn id_key_covers_missing_parsed_and_text() {
        assert_eq!(id_key(None), IdKey::Missing);
        assert_eq!(id_key(Some("7")), IdKey::Parsed(7));
        assert_eq!(id_key(Some("abc")), IdKey::Text("abc".to_string()));
    }

    #[test]
    fn parse_edge_time_handles_none_and_invalid() {
        assert_eq!(parse_edge_time(None), 0);
        assert_eq!(parse_edge_time(Some("nope")), 0);
        assert_eq!(parse_edge_time(Some("9")), 9);
    }

    #[test]
    fn pick_url_winner_can_keep_new() {
        let existing = ArenaNode {
            node_type: "url".to_string(),
            url: Some("https://example.com".to_string()),
            visit_count: Some(1),
            date_last_used: Some("1".to_string()),
            date_added: Some("1".to_string()),
            id: Some("2".to_string()),
            ..ArenaNode::default()
        };
        let new = ArenaNode {
            node_type: "url".to_string(),
            url: Some("https://example.com".to_string()),
            visit_count: Some(10),
            date_last_used: Some("2".to_string()),
            date_added: Some("1".to_string()),
            id: Some("1".to_string()),
            ..ArenaNode::default()
        };

        assert!(matches!(pick_url_winner(&existing, &new), UrlWinner::KeepNew));
    }

    #[tokio::test]
    async fn per_folder_url_dedup_skips_nodes_without_url() {
        let mut arena = Arena::default();
        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            depth: 0,
            path: "/".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(None);
        arena.root_container.insert("bookmark_bar".to_string(), Handle(0));

        arena.nodes.push(ArenaNode {
            node_type: "url".to_string(),
            url: None,
            depth: 1,
            path: "/x".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(Some(Handle(0)));
        arena.nodes[0].children.push(Handle(1));

        let canonicalizer = DefaultUrlCanonicalizer;
        let mut stats = NormalizeStats::default();
        per_folder_url_dedup(&mut arena, &canonicalizer, &None, &mut stats).await;

        assert_eq!(stats.urls_deduped, 0);
        assert!(!arena.nodes[1].deleted);
    }

    #[tokio::test]
    async fn per_folder_url_dedup_skips_deleted_and_non_folder_nodes() {
        let mut arena = Arena::default();

        arena.nodes.push(ArenaNode {
            node_type: "url".to_string(),
            url: Some("https://example.com".to_string()),
            depth: 0,
            path: "/x".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(None);

        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            deleted: true,
            depth: 0,
            path: "/".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(None);

        let canonicalizer = DefaultUrlCanonicalizer;
        let mut stats = NormalizeStats::default();
        per_folder_url_dedup(&mut arena, &canonicalizer, &None, &mut stats).await;

        assert_eq!(stats.urls_deduped, 0);
    }

    #[tokio::test]
    async fn per_folder_url_dedup_emits_event_and_keeps_existing_winner() {
        let mut arena = Arena::default();
        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            depth: 0,
            path: "/".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(None);
        arena.root_container.insert("bookmark_bar".to_string(), Handle(0));

        // Existing winner: higher visit_count.
        arena.nodes.push(ArenaNode {
            node_type: "url".to_string(),
            name: Some("a".to_string()),
            url: Some("https://example.com#frag".to_string()),
            visit_count: Some(10),
            depth: 1,
            path: "/a".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(Some(Handle(0)));
        arena.nodes[0].children.push(Handle(1));

        // New loser: lower visit_count.
        arena.nodes.push(ArenaNode {
            node_type: "url".to_string(),
            name: Some("b".to_string()),
            url: Some("https://example.com".to_string()),
            visit_count: Some(1),
            depth: 1,
            path: "/b".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(Some(Handle(0)));
        arena.nodes[0].children.push(Handle(2));

        let canonicalizer = DefaultUrlCanonicalizer;
        let mut stats = NormalizeStats::default();

        let (tx, mut rx) = mpsc::channel::<AppEvent>(8);
        per_folder_url_dedup(&mut arena, &canonicalizer, &Some(tx), &mut stats).await;

        assert_eq!(stats.urls_deduped, 1);
        assert!(!arena.nodes[1].deleted);
        assert!(arena.nodes[2].deleted);

        let ev = rx.recv().await.expect("event");
        match ev {
            AppEvent::UrlDeduped {
                folder_path,
                canonical_url,
                removed,
            } => {
                assert_eq!(folder_path, "/");
                assert_eq!(canonical_url, "https://example.com");
                assert_eq!(removed, 1);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn per_folder_url_dedup_covers_keep_new_and_keep_existing_paths() {
        let mut arena = Arena::default();
        arena.nodes.push(ArenaNode {
            node_type: "folder".to_string(),
            depth: 0,
            path: "/".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(None);
        arena.root_container.insert("bookmark_bar".to_string(), Handle(0));

        // Initial (will lose to the next one).
        arena.nodes.push(ArenaNode {
            node_type: "url".to_string(),
            url: Some("https://example.com#one".to_string()),
            visit_count: Some(1),
            depth: 1,
            path: "/1".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(Some(Handle(0)));
        arena.nodes[0].children.push(Handle(1));

        // Better (wins) -> triggers KeepNew.
        arena.nodes.push(ArenaNode {
            node_type: "url".to_string(),
            url: Some("https://example.com#two".to_string()),
            visit_count: Some(10),
            depth: 1,
            path: "/2".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(Some(Handle(0)));
        arena.nodes[0].children.push(Handle(2));

        // Worse than current best -> triggers KeepExisting.
        arena.nodes.push(ArenaNode {
            node_type: "url".to_string(),
            url: Some("https://example.com".to_string()),
            visit_count: Some(0),
            depth: 1,
            path: "/3".to_string(),
            ..ArenaNode::default()
        });
        arena.parent.push(Some(Handle(0)));
        arena.nodes[0].children.push(Handle(3));

        let canonicalizer = DefaultUrlCanonicalizer;
        let mut stats = NormalizeStats::default();
        per_folder_url_dedup(&mut arena, &canonicalizer, &None, &mut stats).await;

        assert_eq!(stats.urls_deduped, 2);
        assert!(arena.nodes[1].deleted);
        assert!(!arena.nodes[2].deleted);
        assert!(arena.nodes[3].deleted);
    }
}
