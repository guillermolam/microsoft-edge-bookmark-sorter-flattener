---
agent: "release-engineer-agent"
---

Prepare the repository for a release-quality state and keep iterating until all gates are green and coverage >=98%.

Gates (must be enforced and deterministic):
- cargo fmt --check
- cargo clippy --all-targets --all-features -- -D warnings
- cargo test --all --locked
- coverage >=98% using the canonical command (prefer cargo-llvm-cov)
- optional: cargo nextest run (complements cargo test)

Process loop:
1) Run all gates (or define them if missing).
2) If coverage <98%:
   - compute top 5–10 lowest-covered files/modules
   - trigger handoff to cargo-test-agent with the ranked target list
3) If any gate fails due to:
   - warnings/architecture issues → handoff to rust-architect
   - nondeterminism → handoff to determinism-ordering-agent
   - serde boundary → handoff to serde-compatibility-agent
4) Re-run gates until green.

Output:
- Gates status
- Coverage % and command
- Lowest-covered files/modules (if failing)
- Files changed (paths only)
- Commit hash (`git rev-parse HEAD`) and confirmation it was pushed
- Next handoff invoked (if any)
