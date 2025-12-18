//! Binary entrypoint.
//!
//! This crate is intentionally split into Clean Architecture layers:
//! - domain: pure, synchronous business rules
//! - usecase: orchestration + progress events
//! - infrastructure: serde + async IO + implementations of ports
//! - interface: CLI wiring

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    microsoft_edge_bookmark_sorter_flattener::interface::cli::run().await
}

#[cfg(test)]
mod tests {
    #[test]
    fn main_returns_usage_error_under_test_harness_args() {
        // When executed under `cargo test`, env::args() does not match the CLI contract.
        // We assert a graceful usage error instead of panicking.
        let res = super::main();
        assert!(res.is_err());
    }
}
