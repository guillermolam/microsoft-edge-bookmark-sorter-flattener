mod arena;
mod build;
mod folder_merge;
mod graph;
mod prune;
mod rebuild;
mod url_dedup;

use crate::domain::traits::{SccDetector, UrlCanonicalizer};
use crate::infrastructure::serde_json_adapter::BookmarksFileDto;
use crate::usecase::event::AppEvent;
use crate::usecase::stats::NormalizeStats;
use anyhow::Result;
use serde_json::json;
use tokio::sync::mpsc;

pub async fn normalize_bookmarks(
    input: BookmarksFileDto,
    canonicalizer: &dyn UrlCanonicalizer,
    scc: &dyn SccDetector,
    sink: Option<mpsc::Sender<AppEvent>>,
) -> Result<(BookmarksFileDto, NormalizeStats)> {
    let mut stats = NormalizeStats::default();

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
    let (graph, scc_summary) = graph::build_identity_graph(&arena);
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

    emit(
        &sink,
        AppEvent::PhaseStarted {
            name: "global_folder_merge".into(),
        },
    )
    .await;
    folder_merge::global_folder_merge(&mut arena, &sink, &mut stats).await;
    emit(
        &sink,
        AppEvent::PhaseFinished {
            name: "global_folder_merge".into(),
        },
    )
    .await;

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
            name: "rebuild".into(),
        },
    )
    .await;
    let mut out = rebuild::rebuild_dto_from_arena(input, arena, canonicalizer);
    out.extra
            .insert("x_merge_meta".to_string(), json!({
                "scc": scc_summary
            }));
    emit(
        &sink,
        AppEvent::PhaseFinished {
            name: "rebuild".into(),
        },
    )
    .await;

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
