use microsoft_edge_bookmark_sorter_flattener::infrastructure::scc_kosaraju::KosarajuSccDetector;
use microsoft_edge_bookmark_sorter_flattener::infrastructure::serde_json_adapter::{
    BookmarkNodeDto, BookmarksFileDto,
};
use microsoft_edge_bookmark_sorter_flattener::infrastructure::url_canonicalizer::DefaultUrlCanonicalizer;
use microsoft_edge_bookmark_sorter_flattener::usecase::normalize::normalize_bookmarks;
use microsoft_edge_bookmark_sorter_flattener::usecase::validate::validate_bookmarks;
use std::collections::BTreeMap;

fn root(children: Vec<BookmarkNodeDto>) -> BookmarkNodeDto {
    BookmarkNodeDto {
        node_type: "folder".to_string(),
        name: None,
        url: None,
        children,
        date_added: None,
        date_modified: None,
        date_last_used: None,
        visit_count: None,
        guid: None,
        id: None,
        source: None,
        show_icon: None,
        extra: Default::default(),
    }
}

fn folder(
    name: &str,
    id: Option<&str>,
    guid: Option<&str>,
    children: Vec<BookmarkNodeDto>,
) -> BookmarkNodeDto {
    BookmarkNodeDto {
        node_type: "folder".to_string(),
        name: Some(name.to_string()),
        url: None,
        children,
        date_added: None,
        date_modified: None,
        date_last_used: None,
        visit_count: None,
        guid: guid.map(|s| s.to_string()),
        id: id.map(|s| s.to_string()),
        source: None,
        show_icon: None,
        extra: Default::default(),
    }
}

fn url(name: &str, id: Option<&str>, url: &str) -> BookmarkNodeDto {
    BookmarkNodeDto {
        node_type: "url".to_string(),
        name: Some(name.to_string()),
        url: Some(url.to_string()),
        children: vec![],
        date_added: None,
        date_modified: None,
        date_last_used: None,
        visit_count: None,
        guid: None,
        id: id.map(|s| s.to_string()),
        source: None,
        show_icon: None,
        extra: Default::default(),
    }
}

#[tokio::test]
async fn normalize_output_always_validates() {
    // Construct an input that violates the *post-normalization* invariants:
    // - duplicate folder names across roots
    // - duplicate URLs within a folder (after canonicalization)
    // - empty folders
    // The normalizer should merge/dedup/prune so that validate() succeeds.

    let input = BookmarksFileDto {
        roots: BTreeMap::from([
            (
                "bookmark_bar".to_string(),
                root(vec![
                    folder(
                        "Work",
                        Some("10"),
                        Some("guid-work-1"),
                        vec![
                            url("a", Some("1"), "https://example.com#frag"),
                            url("a-dup", Some("2"), "https://example.com"),
                            folder("Empty", Some("99"), None, vec![]),
                        ],
                    ),
                    folder(
                        "Other",
                        None,
                        None,
                        vec![url("b", None, "https://b.example")],
                    ),
                ]),
            ),
            (
                "other".to_string(),
                root(vec![folder(
                    "  work ",
                    Some("11"),
                    Some("guid-work-2"),
                    vec![url("c", None, "https://c.example")],
                )]),
            ),
        ]),
        ..BookmarksFileDto::default()
    };

    let canonicalizer = DefaultUrlCanonicalizer;
    let scc = KosarajuSccDetector;

    let (out, _stats) = normalize_bookmarks(input, &canonicalizer, &scc, None)
        .await
        .expect("normalize");

    validate_bookmarks(&out, &canonicalizer).expect("normalize output must validate");
}
