use microsoft_edge_bookmark_sorter_flattener::domain::traits::UrlCanonicalizer;
use microsoft_edge_bookmark_sorter_flattener::infrastructure::url_canonicalizer::DefaultUrlCanonicalizer;

#[test]
fn canonicalizer_drops_fragment_and_lowercases_scheme_and_host() {
    let c = DefaultUrlCanonicalizer;

    assert_eq!(
        c.canonicalize(" HTTP://ExAmple.COM/Path#section "),
        "http://example.com/Path"
    );

    // Non-URL strings (no scheme separator) are returned without fragment.
    assert_eq!(c.canonicalize("example.com/x#frag"), "example.com/x");
}
