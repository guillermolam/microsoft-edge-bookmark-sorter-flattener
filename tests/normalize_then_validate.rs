use microsoft_edge_bookmark_sorter_flattener::infrastructure::scc_kosaraju::KosarajuSccDetector;
use microsoft_edge_bookmark_sorter_flattener::infrastructure::serde_json_adapter::read_bookmarks_file;
use microsoft_edge_bookmark_sorter_flattener::infrastructure::url_canonicalizer::DefaultUrlCanonicalizer;
use microsoft_edge_bookmark_sorter_flattener::usecase::normalize::normalize_bookmarks;
use microsoft_edge_bookmark_sorter_flattener::usecase::validate::validate_bookmarks;

#[tokio::test]
async fn normalize_output_always_validates() {
    // Use BookmarksMini which has duplicates and should normalize correctly.
    let input = read_bookmarks_file("tests/resources/BookmarksMini")
        .await
        .expect("read BookmarksMini");

    let canonicalizer = DefaultUrlCanonicalizer;
    let scc = KosarajuSccDetector;

    let (out, _stats) = normalize_bookmarks(input, &canonicalizer, &scc, None)
        .await
        .expect("normalize");

    validate_bookmarks(&out, &canonicalizer).expect("normalize output must validate");
}
