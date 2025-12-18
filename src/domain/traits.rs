use crate::domain::graph::{Graph, SccResult};

pub trait UrlCanonicalizer {
    fn canonicalize(&self, url: &str) -> String;
}

pub trait SccDetector {
    fn compute_scc(&self, graph: &Graph) -> SccResult;
}
