use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use tokio::fs;

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
    fs::write(path, pretty).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn read_write_round_trip_preserves_roots_and_extra() {
        let dir = tempdir().expect("tempdir");
        let input_path = dir.path().join("Bookmarks.json");

        let mut dto = BookmarksFileDto::default();
        dto.version = Some(1);
        dto.extra
            .insert("x_test".to_string(), Value::String("ok".to_string()));
        dto.roots.insert(
            "bookmark_bar".to_string(),
            BookmarkNodeDto {
                node_type: "folder".to_string(),
                name: Some("bar".to_string()),
                ..BookmarkNodeDto::default()
            },
        );

        write_bookmarks_file(input_path.to_str().unwrap(), &dto)
            .await
            .expect("write");

        let reread = read_bookmarks_file(input_path.to_str().unwrap())
            .await
            .expect("read");

        assert_eq!(reread.version, Some(1));
        assert_eq!(reread.extra.get("x_test"), Some(&Value::String("ok".to_string())));
        assert_eq!(reread.roots.len(), 1);
        assert_eq!(
            reread.roots.get("bookmark_bar").and_then(|n| n.name.as_deref()),
            Some("bar")
        );
    }
}
