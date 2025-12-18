---
agent: "release-engineer-agent"
---

Establish the project’s end-to-end quality gates and the canonical coverage workflow that will be used for iterative improvement to >=98% line coverage.

Requirements:
- Define a single canonical local command for coverage (prefer cargo-llvm-cov), including:
  - what is included/excluded
  - how the percentage is computed
  - how to fail the gate if <98%
- Ensure CI parity: the same gates must be runnable in CI and locally.
- CI must treat warnings as errors (clippy -D warnings).
- The coverage workflow must be deterministic across runs.

Deliverables:
1) A documented command set (fmt, clippy, test, coverage) in docs/testing.md and/or docs/coverage.md
2) CI workflow updates under .github/workflows/ that enforce:
   - cargo fmt --check
   - cargo clippy -- -D warnings
   - cargo test
   - coverage >=98%
3) Print in CI logs:
   - current coverage %
   - top 5–10 lowest-covered files/modules when failing (best effort)
4) Commit and push the changes so progress is durable and reviewable

Constraints:
- Do not change domain semantics.
- If coverage cannot be measured reliably, escalate to Plan with clear blockers.

Once tooling is in place:
- Run the canonical coverage command and report the baseline coverage % and lowest-covered modules.
- If baseline <98%, trigger your handoff to cargo-test-agent with the ranked target list.

Output (add to the report):
- Commit hash (`git rev-parse HEAD`) and confirmation it was pushed
