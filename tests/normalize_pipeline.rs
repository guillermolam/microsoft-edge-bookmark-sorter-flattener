use microsoft_edge_bookmark_sorter_flattener::infrastructure::scc_kosaraju::KosarajuSccDetector;
use microsoft_edge_bookmark_sorter_flattener::infrastructure::serde_json_adapter::{
    BookmarkNodeDto, BookmarksFileDto,
};
use microsoft_edge_bookmark_sorter_flattener::infrastructure::url_canonicalizer::DefaultUrlCanonicalizer;
use microsoft_edge_bookmark_sorter_flattener::usecase::event::AppEvent;
use microsoft_edge_bookmark_sorter_flattener::usecase::normalize::normalize_bookmarks;
use serde_json::json;
use std::collections::BTreeMap;
use tokio::sync::mpsc;

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
    date_added: Option<&str>,
    children: Vec<BookmarkNodeDto>,
) -> BookmarkNodeDto {
    BookmarkNodeDto {
        node_type: "folder".to_string(),
        name: Some(name.to_string()),
        url: None,
        children,
        date_added: date_added.map(|s| s.to_string()),
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

fn url(
    name: &str,
    id: Option<&str>,
    url: &str,
    visit_count: Option<i64>,
    date_last_used: Option<&str>,
    date_added: Option<&str>,
) -> BookmarkNodeDto {
    BookmarkNodeDto {
        node_type: "url".to_string(),
        name: Some(name.to_string()),
        url: Some(url.to_string()),
        children: vec![],
        date_added: date_added.map(|s| s.to_string()),
        date_modified: None,
        date_last_used: date_last_used.map(|s| s.to_string()),
        visit_count,
        guid: None,
        id: id.map(|s| s.to_string()),
        source: None,
        show_icon: None,
        extra: Default::default(),
    }
}

fn separator(name: &str) -> BookmarkNodeDto {
    BookmarkNodeDto {
        node_type: "separator".to_string(),
        name: Some(name.to_string()),
        url: None,
        children: vec![],
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

fn mk_input(roots: Vec<(&str, BookmarkNodeDto)>) -> BookmarksFileDto {
    let mut map = BTreeMap::new();
    for (k, v) in roots {
        map.insert(k.to_string(), v);
    }
    BookmarksFileDto {
        checksum: None,
        roots: map,
        version: None,
        extra: Default::default(),
    }
}

fn traverse<'a>(root: &'a BookmarkNodeDto) -> Vec<&'a BookmarkNodeDto> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(n) = stack.pop() {
        out.push(n);
        for ch in n.children.iter().rev() {
            stack.push(ch);
        }
    }
    out
}

fn find_folders_named<'a>(dto: &'a BookmarksFileDto, name: &str) -> Vec<&'a BookmarkNodeDto> {
    let mut found = Vec::new();
    for root in dto.roots.values() {
        for n in traverse(root) {
            if n.node_type == "folder" && n.name.as_deref() == Some(name) {
                found.push(n);
            }
        }
    }
    found
}

fn find_urls_in_folder<'a>(folder: &'a BookmarkNodeDto) -> Vec<&'a BookmarkNodeDto> {
    folder
        .children
        .iter()
        .filter(|n| n.node_type == "url")
        .collect()
}

#[tokio::test]
async fn global_folder_merge_outermost_wins_and_records_meta() {
    // bookmark_bar:
    //   0: J
    //      0: Z (loser)
    //   1: Z (winner)
    let loser_z = folder(
        "Z",
        Some("2"),
        Some("guid-z-loser"),
        Some("200"),
        vec![url(
            "a",
            Some("10"),
            "http://example.com/a",
            None,
            None,
            Some("10"),
        )],
    );
    let j = folder(
        "J",
        Some("20"),
        None,
        Some("150"),
        vec![
            loser_z,
            url(
                "j",
                Some("11"),
                "http://example.com/j",
                None,
                None,
                Some("11"),
            ),
        ],
    );
    let winner_z = folder(
        "Z",
        Some("1"),
        Some("guid-z-winner"),
        Some("100"),
        vec![url(
            "b",
            Some("12"),
            "http://example.com/b",
            None,
            None,
            Some("12"),
        )],
    );

    let input = mk_input(vec![("bookmark_bar", root(vec![j, winner_z]))]);

    let canonicalizer = DefaultUrlCanonicalizer;
    let scc = KosarajuSccDetector;

    let (out, stats) = normalize_bookmarks(input, &canonicalizer, &scc, None)
        .await
        .expect("normalize_bookmarks should succeed");

    assert_eq!(stats.folders_merged, 1);

    let zs = find_folders_named(&out, "Z");
    assert_eq!(zs.len(), 1, "global merge should leave exactly one Z");

    let z = zs[0];
    let urls = find_urls_in_folder(z);
    let mut url_vals: Vec<String> = urls
        .iter()
        .map(|u| u.url.clone().unwrap_or_default())
        .collect();
    url_vals.sort();
    assert_eq!(
        url_vals,
        vec![
            "http://example.com/a".to_string(),
            "http://example.com/b".to_string()
        ]
    );

    let meta = z
        .extra
        .get("x_merge_meta")
        .cloned()
        .expect("winner folder should have x_merge_meta");

    assert!(
        meta.get("merged_paths").is_some(),
        "x_merge_meta should contain merged_paths"
    );
    let merged_paths = meta["merged_paths"].as_array().cloned().unwrap_or_default();
    let merged_paths_str: Vec<String> = merged_paths
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    assert!(
        merged_paths_str
            .iter()
            .any(|p| p.ends_with("bookmark_bar/0/0")),
        "merged_paths should include loser folder path"
    );
}

#[tokio::test]
async fn url_dedup_keeps_best_and_records_merged_from() {
    let a = folder(
        "A",
        Some("1"),
        None,
        Some("100"),
        vec![
            url(
                "keep",
                Some("1"),
                "http://EXAMPLE.com/page#frag",
                Some(10),
                Some("500"),
                Some("100"),
            ),
            url(
                "lose",
                Some("2"),
                "http://example.com/page",
                Some(1),
                Some("400"),
                Some("200"),
            ),
        ],
    );

    let input = mk_input(vec![("bookmark_bar", root(vec![a]))]);

    let canonicalizer = DefaultUrlCanonicalizer;
    let scc = KosarajuSccDetector;

    let (out, stats) = normalize_bookmarks(input, &canonicalizer, &scc, None)
        .await
        .expect("normalize_bookmarks should succeed");

    assert_eq!(stats.urls_deduped, 1);

    let a_out = find_folders_named(&out, "A");
    assert_eq!(a_out.len(), 1);
    let urls = find_urls_in_folder(a_out[0]);
    assert_eq!(urls.len(), 1);

    let winner = urls[0];
    assert_eq!(
        winner.id.as_deref(),
        Some("1"),
        "winner should be the higher visit_count URL"
    );

    let meta = winner
        .extra
        .get("x_merge_meta")
        .cloned()
        .expect("winner URL should have x_merge_meta");

    let merged_from = meta["merged_from"].as_array().cloned().unwrap_or_default();
    assert_eq!(merged_from.len(), 1);
    assert_eq!(merged_from[0]["id"].as_str(), Some("2"));
}

#[tokio::test]
async fn prune_removes_empty_folders() {
    let empty = folder("Empty", Some("1"), None, Some("100"), vec![]);
    let input = mk_input(vec![("bookmark_bar", root(vec![empty]))]);

    let canonicalizer = DefaultUrlCanonicalizer;
    let scc = KosarajuSccDetector;

    let (out, stats) = normalize_bookmarks(input, &canonicalizer, &scc, None)
        .await
        .expect("normalize_bookmarks should succeed");

    assert_eq!(stats.folders_pruned, 1);
    assert!(find_folders_named(&out, "Empty").is_empty());
}

#[tokio::test]
async fn rebuild_sorts_children_deterministically() {
    let order = folder(
        "Order",
        Some("1"),
        None,
        Some("100"),
        vec![
            url("b", Some("4"), "http://B.com/x", None, None, Some("10")),
            folder("b", Some("3"), None, Some("10"), vec![]),
            separator("sep"),
            folder("A", Some("2"), None, Some("10"), vec![]),
            url("a", Some("5"), "http://a.com/x", None, None, Some("10")),
        ],
    );

    let input = mk_input(vec![("bookmark_bar", root(vec![order]))]);

    let canonicalizer = DefaultUrlCanonicalizer;
    let scc = KosarajuSccDetector;

    let (out, _stats) = normalize_bookmarks(input, &canonicalizer, &scc, None)
        .await
        .expect("normalize_bookmarks should succeed");

    let order_out = find_folders_named(&out, "Order");
    assert_eq!(order_out.len(), 1);

    let kids = &order_out[0].children;
    // Empty folders are pruned, so only URLs + other nodes remain.
    assert_eq!(kids.len(), 3);

    // urls first (canonical URL order: a.com then b.com)
    assert_eq!(kids[0].node_type, "url");
    assert_eq!(kids[0].url.as_deref(), Some("http://a.com/x"));
    assert_eq!(kids[1].node_type, "url");
    assert_eq!(kids[1].url.as_deref(), Some("http://B.com/x"));

    // other types last
    assert_eq!(kids[2].node_type, "separator");
}

#[tokio::test]
async fn preserves_unknown_fields_and_adds_top_level_merge_meta() {
    let mut u = url(
        "x",
        Some("1"),
        "http://example.com/x",
        None,
        None,
        Some("1"),
    );
    u.extra.insert("custom".to_string(), json!("keep_me"));

    let f = folder("F", Some("1"), None, Some("1"), vec![u]);

    let mut input = mk_input(vec![("bookmark_bar", root(vec![f]))]);
    input.extra.insert("top".to_string(), json!("keep_top"));

    let canonicalizer = DefaultUrlCanonicalizer;
    let scc = KosarajuSccDetector;

    let (out, _stats) = normalize_bookmarks(input, &canonicalizer, &scc, None)
        .await
        .expect("normalize_bookmarks should succeed");

    assert_eq!(out.extra.get("top"), Some(&json!("keep_top")));
    assert!(
        out.extra.get("x_merge_meta").is_some(),
        "normalize should always add top-level x_merge_meta"
    );

    let f_out = find_folders_named(&out, "F");
    assert_eq!(f_out.len(), 1);
    let urls = find_urls_in_folder(f_out[0]);
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].extra.get("custom"), Some(&json!("keep_me")));
}

#[tokio::test]
async fn emits_scc_event_with_cyclic_component_count() {
    // Create a cycle in the identity graph by repeating GUIDs in a nested structure:
    // root -> A(guid=1) -> B(guid=2) -> A2(guid=1)
    let a2 = folder("A", None, Some("g1"), Some("3"), vec![]);
    let b = folder("B", None, Some("g2"), Some("2"), vec![a2]);
    let a1 = folder("A", None, Some("g1"), Some("1"), vec![b]);

    let input = mk_input(vec![("bookmark_bar", root(vec![a1]))]);

    let canonicalizer = DefaultUrlCanonicalizer;
    let scc = KosarajuSccDetector;

    let (tx, mut rx) = mpsc::channel::<AppEvent>(128);

    let (_out, _stats) = normalize_bookmarks(input, &canonicalizer, &scc, Some(tx))
        .await
        .expect("normalize_bookmarks should succeed");

    let mut scc_computed: Option<AppEvent> = None;
    while let Some(ev) = rx.recv().await {
        if matches!(ev, AppEvent::SccComputed { .. }) {
            scc_computed = Some(ev);
        }
    }

    let AppEvent::SccComputed {
        nodes,
        edges,
        components,
        cyclic_components,
    } = scc_computed.expect("should emit SccComputed")
    else {
        unreachable!();
    };

    // root container adds one identity node; the A/B cycle adds two more.
    assert_eq!(nodes, 3);
    assert_eq!(edges, 3, "root->A plus A<->B cycle");
    assert_eq!(cyclic_components, 1);
    assert_eq!(components, 2);
}
