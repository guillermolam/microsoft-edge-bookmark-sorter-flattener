use crate::domain::traits::UrlCanonicalizer;

pub struct DefaultUrlCanonicalizer;

impl UrlCanonicalizer for DefaultUrlCanonicalizer {
    fn canonicalize(&self, url: &str) -> String {
        // Conservative, dependency-free canonicalization:
        // - trim
        // - drop fragment (#...)
        // - lowercase scheme and host for http(s) URLs
        let trimmed = url.trim();
        let (no_frag, _frag) = match trimmed.split_once('#') {
            Some((a, b)) => (a, Some(b)),
            None => (trimmed, None),
        };

        let Some((scheme, rest)) = no_frag.split_once("://") else {
            return no_frag.to_string();
        };

        let scheme_lc = scheme.to_lowercase();

        // authority is up to first / ?
        let mut end = rest.len();
        for (i, ch) in rest.char_indices() {
            if ch == '/' || ch == '?' {
                end = i;
                break;
            }
        }

        let authority = &rest[..end];
        let tail = &rest[end..];
        let authority_lc = authority.to_lowercase();
        format!("{scheme_lc}://{authority_lc}{tail}")
    }
}
