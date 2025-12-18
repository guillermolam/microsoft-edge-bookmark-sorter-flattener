use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use tokio::{fs, process, time};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookmarksFileDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,

    #[serde(default)]
    pub roots: BTreeMap<String, BookmarkNodeDto>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<i64>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookmarkNodeDto {
    #[serde(rename = "type")]
    pub node_type: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<BookmarkNodeDto>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_added: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_modified: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_last_used: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visit_count: Option<i64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_icon: Option<bool>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

pub async fn read_bookmarks_file(path: &str) -> Result<BookmarksFileDto> {
    let raw = fs::read_to_string(path).await?;
    let dto: BookmarksFileDto = serde_json::from_str(&raw)?;
    Ok(dto)
}

pub async fn write_bookmarks_file(path: &str, dto: &BookmarksFileDto) -> Result<()> {
    let pretty = serde_json::to_string_pretty(dto)?;

    let max_retries = 5;
    let mut last_error = None;

    for attempt in 1..=max_retries {
        match fs::write(path, &pretty).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = Some(e.to_string());

                // Check if it's a permission/lock error
                if e.kind() == std::io::ErrorKind::PermissionDenied
                    || e.raw_os_error()
                        .map_or(false, |code| code == 13 || code == 1)
                {
                    // EACCES or EPERM

                    if attempt == 1 {
                        eprintln!("Warning: File {} appears to be locked or inaccessible (possibly by Microsoft Edge).", path);
                        eprintln!("Attempting to close Microsoft Edge processes...");

                        // Try to find and terminate Microsoft Edge processes
                        let _ = process::Command::new("pkill")
                            .args(&["-f", "microsoft-edge"])
                            .status()
                            .await;

                        let _ = process::Command::new("pkill")
                            .args(&["-f", "msedge"])
                            .status()
                            .await;

                        // Wait a bit for processes to terminate
                        time::sleep(time::Duration::from_millis(500)).await;
                    }

                    if attempt < max_retries {
                        let delay_ms = 500 * attempt; // Progressive delay: 500ms, 1000ms, 1500ms, 2000ms
                        eprintln!("Retry {} of {} in {}ms...", attempt, max_retries, delay_ms);
                        time::sleep(time::Duration::from_millis(delay_ms)).await;
                        continue;
                    }
                } else {
                    // Not a permission error, fail immediately
                    return Err(e.into());
                }
            }
        }
    }

    // If we get here, all retries failed
    Err(std::io::Error::new(std::io::ErrorKind::Other, last_error.unwrap()).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn read_write_round_trip_preserves_roots_and_extra() {
        let dir = tempdir().expect("tempdir");
        let input_path = dir.path().join("Bookmarks.json");

        let dto = BookmarksFileDto {
            version: Some(1),
            extra: BTreeMap::from([("x_test".to_string(), Value::String("ok".to_string()))]),
            roots: BTreeMap::from([(
                "bookmark_bar".to_string(),
                BookmarkNodeDto {
                    node_type: "folder".to_string(),
                    name: Some("bar".to_string()),
                    ..BookmarkNodeDto::default()
                },
            )]),
            ..BookmarksFileDto::default()
        };

        write_bookmarks_file(input_path.to_str().unwrap(), &dto)
            .await
            .expect("write");

        let reread = read_bookmarks_file(input_path.to_str().unwrap())
            .await
            .expect("read");

        assert_eq!(reread.version, Some(1));
        assert_eq!(
            reread.extra.get("x_test"),
            Some(&Value::String("ok".to_string()))
        );
        assert_eq!(reread.roots.len(), 1);
        assert_eq!(
            reread
                .roots
                .get("bookmark_bar")
                .and_then(|n| n.name.as_deref()),
            Some("bar")
        );
    }
}
