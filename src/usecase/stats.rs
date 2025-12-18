use serde::Serialize;

#[derive(Debug, Clone, Default, Serialize)]
pub struct NormalizeStats {
    pub folders_seen: usize,
    pub folders_merged: usize,
    pub urls_seen: usize,
    pub urls_deduped: usize,
    pub folders_pruned: usize,
}
