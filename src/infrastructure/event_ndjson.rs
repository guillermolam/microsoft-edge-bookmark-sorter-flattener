use crate::usecase::event::AppEvent;
use serde_json::json;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

fn app_event_to_json(ev: &AppEvent) -> serde_json::Value {
    match ev {
        AppEvent::PhaseStarted { name } => json!({"type":"phase_started","name":name}),
        AppEvent::PhaseFinished { name } => json!({"type":"phase_finished","name":name}),
        AppEvent::SccComputed {
            nodes,
            edges,
            components,
            cyclic_components,
        } => {
            json!({"type":"scc_computed","nodes":nodes,"edges":edges,"components":components,"cyclic_components":cyclic_components})
        }
        AppEvent::FolderMergePlanned {
            normalized_name,
            group_size,
        } => {
            json!({"type":"folder_merge_planned","normalized_name":normalized_name,"group_size":group_size})
        }
        AppEvent::FolderMerged {
            normalized_name,
            winner_path,
            losers,
        } => {
            json!({"type":"folder_merged","normalized_name":normalized_name,"winner_path":winner_path,"losers":losers})
        }
        AppEvent::UrlDeduped {
            folder_path,
            canonical_url,
            removed,
        } => {
            json!({"type":"url_deduped","folder_path":folder_path,"canonical_url":canonical_url,"removed":removed})
        }
        AppEvent::FolderPruned { folder_path } => {
            json!({"type":"folder_pruned","folder_path":folder_path})
        }
        AppEvent::Finished { stats } => json!({"type":"finished","stats":stats}),
    }
}

pub fn spawn_ndjson_printer(mut rx: mpsc::Receiver<AppEvent>) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(ev) = rx.recv().await {
            let line = app_event_to_json(&ev);

            // NDJSON to stdout.
            println!("{line}");
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usecase::stats::NormalizeStats;

    #[test]
    fn app_event_to_json_covers_all_variants() {
        let v = app_event_to_json(&AppEvent::PhaseStarted {
            name: "x".to_string(),
        });
        assert_eq!(v["type"], "phase_started");

        let v = app_event_to_json(&AppEvent::PhaseFinished {
            name: "x".to_string(),
        });
        assert_eq!(v["type"], "phase_finished");

        let v = app_event_to_json(&AppEvent::SccComputed {
            nodes: 1,
            edges: 2,
            components: 3,
            cyclic_components: 4,
        });
        assert_eq!(v["type"], "scc_computed");
        assert_eq!(v["nodes"], 1);

        let v = app_event_to_json(&AppEvent::FolderMergePlanned {
            normalized_name: "a".to_string(),
            group_size: 2,
        });
        assert_eq!(v["type"], "folder_merge_planned");

        let v = app_event_to_json(&AppEvent::FolderMerged {
            normalized_name: "a".to_string(),
            winner_path: "/root/a".to_string(),
            losers: vec!["/root/b".to_string()],
        });
        assert_eq!(v["type"], "folder_merged");

        let v = app_event_to_json(&AppEvent::UrlDeduped {
            folder_path: "/root".to_string(),
            canonical_url: "https://example.com".to_string(),
            removed: 1,
        });
        assert_eq!(v["type"], "url_deduped");

        let v = app_event_to_json(&AppEvent::FolderPruned {
            folder_path: "/root/empty".to_string(),
        });
        assert_eq!(v["type"], "folder_pruned");

        let v = app_event_to_json(&AppEvent::Finished {
            stats: NormalizeStats::default(),
        });
        assert_eq!(v["type"], "finished");
    }

    #[tokio::test]
    async fn spawn_ndjson_printer_drains_and_exits() {
        let (tx, rx) = mpsc::channel::<AppEvent>(8);
        let handle = spawn_ndjson_printer(rx);

        tx.send(AppEvent::PhaseStarted {
            name: "x".to_string(),
        })
        .await
        .expect("send");
        drop(tx);

        handle.await.expect("join");
    }
}
