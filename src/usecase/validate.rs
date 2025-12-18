use crate::domain::traits::UrlCanonicalizer;
use crate::infrastructure::serde_json_adapter::BookmarksFileDto;
use anyhow::{anyhow, Result};
use std::collections::{BTreeMap, BTreeSet};

pub fn validate_bookmarks(
    dto: &BookmarksFileDto,
    canonicalizer: &dyn UrlCanonicalizer,
) -> Result<()> {
    // Iterative traversal (no recursion).
    // We treat the document as a forest of folders rooted at `dto.roots`.

    // Global uniqueness by normalized folder name.
    let mut global_folder_owner: BTreeMap<String, String> = BTreeMap::new();

    // Stack holds (path, node, is_root_container).
    let mut stack: Vec<(
        String,
        &crate::infrastructure::serde_json_adapter::BookmarkNodeDto,
        bool,
    )> = Vec::new();

    for (root_key, root) in dto.roots.iter() {
        stack.push((format!("/{root_key}"), root, true));
    }

    while let Some((path, node, is_root_container)) = stack.pop() {
        if node.node_type != "folder" {
            continue;
        }

        if let Some(name) = node.name.as_ref() {
            let norm = name.trim().to_lowercase();
            if let Some(existing_path) = global_folder_owner.get(&norm) {
                return Err(anyhow!(
                    "folder name must be globally unique: {norm} (saw at {existing_path} and {path})"
                ));
            }
            global_folder_owner.insert(norm, path.clone());
        }

        // Empty folders removed (root containers are allowed to be empty).
        if !is_root_container && node.children.is_empty() {
            return Err(anyhow!("empty folder found at {path}"));
        }

        // No duplicate subfolder names within the same folder.
        let mut seen_child_folders: BTreeSet<String> = BTreeSet::new();

        // URL dedup per folder.
        let mut seen_urls: BTreeSet<String> = BTreeSet::new();

        for child in node.children.iter() {
            if child.node_type == "folder" {
                if let Some(n) = child.name.as_ref() {
                    let norm = n.trim().to_lowercase();
                    if !seen_child_folders.insert(norm.clone()) {
                        return Err(anyhow!("duplicate subfolder name under {path}: {norm}"));
                    }
                }
            }

            if child.node_type == "url" {
                if let Some(url) = child.url.as_ref() {
                    let canon = canonicalizer.canonicalize(url);
                    if !seen_urls.insert(canon.clone()) {
                        return Err(anyhow!("duplicate URL under {path}: {canon}"));
                    }
                }
            }
        }

        // Push children deterministically.
        // `children` is a Vec from JSON; we preserve order but include index in fallback path segment.
        for (idx, child) in node.children.iter().enumerate().rev() {
            let seg = child
                .name
                .clone()
                .unwrap_or_else(|| format!("{}[{idx}]", child.node_type));
            stack.push((format!("{path}/{seg}"), child, false));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::serde_json_adapter::BookmarkNodeDto;
    use crate::infrastructure::url_canonicalizer::DefaultUrlCanonicalizer;
    use std::collections::BTreeMap;

    fn mk_folder(name: &str, children: Vec<BookmarkNodeDto>) -> BookmarkNodeDto {
        BookmarkNodeDto {
            node_type: "folder".to_string(),
            name: Some(name.to_string()),
            children,
            ..BookmarkNodeDto::default()
        }
    }

    fn mk_url(url: &str) -> BookmarkNodeDto {
        BookmarkNodeDto {
            node_type: "url".to_string(),
            url: Some(url.to_string()),
            ..BookmarkNodeDto::default()
        }
    }

    #[test]
    fn validate_accepts_minimal_valid_forest() {
        let dto = BookmarksFileDto {
            roots: BTreeMap::from([(
                "bookmark_bar".to_string(),
                mk_folder("bar", vec![mk_url("https://example.com")]),
            )]),
            ..BookmarksFileDto::default()
        };

        let canonicalizer = DefaultUrlCanonicalizer;
        validate_bookmarks(&dto, &canonicalizer).expect("valid");
    }

    #[test]
    fn validate_rejects_global_duplicate_folder_names() {
        let dto = BookmarksFileDto {
            roots: BTreeMap::from([
                (
                    "bookmark_bar".to_string(),
                    mk_folder("Dup", vec![mk_url("https://a")]),
                ),
                (
                    "other".to_string(),
                    mk_folder("dup", vec![mk_url("https://b")]),
                ),
            ]),
            ..BookmarksFileDto::default()
        };

        let canonicalizer = DefaultUrlCanonicalizer;
        let err = validate_bookmarks(&dto, &canonicalizer)
            .unwrap_err()
            .to_string();
        assert!(err.contains("globally unique"));
    }

    #[test]
    fn validate_rejects_duplicate_subfolder_names_within_folder() {
        let dto = BookmarksFileDto {
            roots: BTreeMap::from([(
                "bookmark_bar".to_string(),
                mk_folder(
                    "bar",
                    vec![
                        mk_folder("X", vec![mk_url("https://a")]),
                        mk_folder("x", vec![mk_url("https://b")]),
                    ],
                ),
            )]),
            ..BookmarksFileDto::default()
        };

        let canonicalizer = DefaultUrlCanonicalizer;
        let err = validate_bookmarks(&dto, &canonicalizer)
            .unwrap_err()
            .to_string();
        assert!(err.contains("duplicate subfolder"));
    }

    #[test]
    fn validate_rejects_empty_non_root_folder() {
        let dto = BookmarksFileDto {
            roots: BTreeMap::from([(
                "bookmark_bar".to_string(),
                mk_folder("bar", vec![mk_folder("empty", vec![])]),
            )]),
            ..BookmarksFileDto::default()
        };

        let canonicalizer = DefaultUrlCanonicalizer;
        let err = validate_bookmarks(&dto, &canonicalizer)
            .unwrap_err()
            .to_string();
        assert!(err.contains("empty folder"));
    }

    #[test]
    fn validate_rejects_duplicate_urls_within_folder() {
        let dto = BookmarksFileDto {
            roots: BTreeMap::from([(
                "bookmark_bar".to_string(),
                mk_folder(
                    "bar",
                    vec![
                        mk_url("https://EXAMPLE.com#frag"),
                        mk_url("https://example.com"),
                    ],
                ),
            )]),
            ..BookmarksFileDto::default()
        };

        let canonicalizer = DefaultUrlCanonicalizer;
        let err = validate_bookmarks(&dto, &canonicalizer)
            .unwrap_err()
            .to_string();
        assert!(err.contains("duplicate URL"));
    }
}
