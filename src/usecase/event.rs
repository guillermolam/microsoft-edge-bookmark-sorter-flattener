use crate::usecase::stats::NormalizeStats;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum AppEvent {
    PhaseStarted {
        name: String,
    },
    PhaseFinished {
        name: String,
    },

    SccComputed {
        nodes: usize,
        edges: usize,
        components: usize,
        cyclic_components: usize,
    },

    FolderMergePlanned {
        normalized_name: String,
        group_size: usize,
    },

    FolderMerged {
        normalized_name: String,
        winner_path: String,
        losers: Vec<String>,
    },

    UrlDeduped {
        folder_path: String,
        canonical_url: String,
        removed: usize,
    },

    FolderPruned {
        folder_path: String,
    },

    Finished {
        stats: NormalizeStats,
    },
}
