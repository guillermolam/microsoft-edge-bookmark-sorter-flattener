---
agent: "Plan"
---

A test or CI gate is failing. Route the failure to the correct owning agent and iterate until green, without weakening invariants.

Process:
1) Run cargo-test-agent to produce a Failure Brief ONLY:
   - failing test(s) or failing gate
   - expected vs actual
   - likely responsible file(s)/symbol(s)
   - classification:
     (A) domain semantic ambiguity
     (B) implementation/architecture bug or warnings-as-errors
     (C) determinism/ordering instability
     (D) SCC/cycle safety issue
     (E) serde/DTO boundary issue
     (F) async boundary/CLI wiring issue

2) Delegate exactly one fix owner:
   - (A) domain-guardian
   - (B) rust-architect
   - (C) determinism-ordering-agent
   - (D) graph-scc-specialist
   - (E) serde-compatibility-agent
   - (F) tokio-cli-ergonomics-agent

3) Rerun cargo-test-agent.
4) Repeat until:
   - cargo test is green
   - and any CI lint gate is green (fmt/clippy -D warnings)

Rules:
- Do not “test current buggy behavior”.
- Do not bypass with broad #[allow] unless removal is impossible and justified.
- Keep diffs minimal.
- If semantics must change, domain-guardian must explicitly approve.

Output:
- Root cause classification
- Owning agent that fixed it
- Files changed (paths only)
- Confirmation gates are green
