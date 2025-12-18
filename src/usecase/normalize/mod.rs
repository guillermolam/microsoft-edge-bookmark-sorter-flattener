mod arena;
mod build;
mod folder_merge;
mod graph;
mod prune;
mod rebuild;
mod url_dedup;

use crate::domain::traits::{SccDetector, UrlCanonicalizer};
use crate::infrastructure::serde_json_adapter::{BookmarkNodeDto, BookmarksFileDto};
use crate::usecase::event::AppEvent;
use crate::usecase::stats::NormalizeStats;
use anyhow::Result;
use std::collections::HashMap;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct FolderRegistry {
    pub counts: HashMap<String, usize>,
}

impl FolderRegistry {
    pub fn new() -> Self {
        Self {
            counts: HashMap::new(),
        }
    }

    pub fn count_folders(&mut self, dto: &BookmarksFileDto) {
        self.counts.clear();
        for root in dto.roots.values() {
            self.traverse_and_count(root);
        }
    }

    fn traverse_and_count(&mut self, node: &BookmarkNodeDto) {
        if node.node_type == "folder" {
            if let Some(name) = &node.name {
                *self.counts.entry(name.clone()).or_insert(0) += 1;
            }
        }
        for child in &node.children {
            self.traverse_and_count(child);
        }
    }

    pub fn all_unique(&self) -> bool {
        self.counts.values().all(|&count| count == 1)
    }

    pub fn print_counts(&self) {
        let mut sorted: Vec<_> = self.counts.iter().collect();
        sorted.sort_by_key(|(name, _)| (*name).clone());
        eprintln!("Folder counts:");
        for (name, count) in sorted {
            eprintln!("  {name}: {count}");
        }
        let all = self.all_unique();
        eprintln!("All unique: {all}");
    }

    pub fn print_final_registry(&self) {
        let mut sorted: Vec<_> = self.counts.iter().collect();
        sorted.sort_by_key(|(name, _)| (*name).clone());
        eprintln!("\nFinal Folder Registry:");
        eprintln!("folder_name | count (number of occurrences)");
        eprintln!("------------|-----------------------------");
        for (name, count) in sorted {
            eprintln!("{name:<12} | {count} (number of occurrences)");
        }
        let all = self.all_unique();
        eprintln!("All unique: {all}");
    }
}

impl Default for FolderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn normalize_bookmarks(
    input: BookmarksFileDto,
    canonicalizer: &dyn UrlCanonicalizer,
    scc: &dyn SccDetector,
    sink: Option<mpsc::Sender<AppEvent>>,
) -> Result<(BookmarksFileDto, NormalizeStats)> {
    let mut stats = NormalizeStats::default();
    let mut registry = FolderRegistry::new();

    emit(
        &sink,
        AppEvent::PhaseStarted {
            name: "parse_and_index".into(),
        },
    )
    .await;
    let mut arena = build::build_arena_from_dto(&input, &mut stats);
    emit(
        &sink,
        AppEvent::PhaseFinished {
            name: "parse_and_index".into(),
        },
    )
    .await;

    emit(&sink, AppEvent::PhaseStarted { name: "scc".into() }).await;
    let (graph, _scc_summary) = graph::build_identity_graph(&arena);
    let scc_res = scc.compute_scc(&graph);
    emit(
        &sink,
        AppEvent::SccComputed {
            nodes: graph.node_count(),
            edges: graph.edge_count(),
            components: scc_res.components.len(),
            cyclic_components: scc_res.cyclic_component.iter().filter(|&&b| b).count(),
        },
    )
    .await;
    emit(&sink, AppEvent::PhaseFinished { name: "scc".into() }).await;

    // Iterative folder merging until all folders are unique
    let mut iteration = 0;
    loop {
        iteration += 1;
        emit(
            &sink,
            AppEvent::PhaseStarted {
                name: format!("folder_merge_iteration_{iteration}"),
            },
        )
        .await;

        folder_merge::global_folder_merge(&mut arena, &sink, &mut stats).await;

        emit(
            &sink,
            AppEvent::PhaseStarted {
                name: "per_folder_url_dedup".into(),
            },
        )
        .await;
        url_dedup::per_folder_url_dedup(&mut arena, canonicalizer, &sink, &mut stats).await;
        emit(
            &sink,
            AppEvent::PhaseFinished {
                name: "per_folder_url_dedup".into(),
            },
        )
        .await;

        emit(
            &sink,
            AppEvent::PhaseStarted {
                name: "prune_empty".into(),
            },
        )
        .await;
        prune::prune_empty_folders(&mut arena, &sink, &mut stats).await;
        emit(
            &sink,
            AppEvent::PhaseFinished {
                name: "prune_empty".into(),
            },
        )
        .await;

        emit(
            &sink,
            AppEvent::PhaseStarted {
                name: "rebuild_check".into(),
            },
        )
        .await;
        let current_dto =
            rebuild::rebuild_dto_from_arena(input.clone(), arena.clone(), canonicalizer);
        registry.count_folders(&current_dto);
        registry.print_counts();

        emit(
            &sink,
            AppEvent::FolderCounts {
                counts: registry.counts.clone(),
            },
        )
        .await;

        if registry.all_unique() {
            emit(
                &sink,
                AppEvent::PhaseFinished {
                    name: format!("folder_merge_iteration_{iteration}_complete"),
                },
            )
            .await;
            break;
        }

        emit(
            &sink,
            AppEvent::PhaseFinished {
                name: format!("folder_merge_iteration_{iteration}_continue"),
            },
        )
        .await;
    }

    emit(
        &sink,
        AppEvent::PhaseStarted {
            name: "final_rebuild".into(),
        },
    )
    .await;
    let out = rebuild::rebuild_dto_from_arena(input, arena, canonicalizer);
    // Removed x_merge_meta to preserve original JSON structure
    emit(
        &sink,
        AppEvent::PhaseFinished {
            name: "final_rebuild".into(),
        },
    )
    .await;

    // Print final registry
    registry.print_final_registry();

    emit(
        &sink,
        AppEvent::Finished {
            stats: stats.clone(),
        },
    )
    .await;
    Ok((out, stats))
}

async fn emit(sink: &Option<mpsc::Sender<AppEvent>>, ev: AppEvent) {
    if let Some(tx) = sink {
        let _ = tx.send(ev).await;
    }
}
