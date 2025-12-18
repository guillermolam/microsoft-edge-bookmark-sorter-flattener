---
agent: "rust-architect"
---

Perform an architecture audit and refactor loop until Clean Architecture boundaries and Rust-idiomatic patterns are restored, without changing domain semantics.

Targets:
- eliminate layering violations (serde in domain, async in domain, infra leaks)
- remove “Java style” patterns (manager/service classes without domain meaning)
- ensure cycle-safety preprocessing exists (SCC-first or delegated)
- ensure deterministic output is enforceable (ordering utilities exist or delegated)

Process:
1) Identify boundary violations and duplication (paths + symbols).
2) Apply minimal refactors to restore:
   - domain purity (no serde, no async)
   - usecase orchestration
   - infra adapters
   - interface runtime/CLI
3) If blocked:
   - domain semantics unclear → handoff to domain-guardian
   - determinism rules needed → handoff to determinism-ordering-agent
   - SCC/cycle design needed → handoff to graph-scc-specialist
   - DTO boundary needed → handoff to serde-compatibility-agent
4) Ensure warning-free build (CI -D warnings).
5) Run cargo test and report.
6) Commit and push the changes so progress is durable and reviewable.

Output:
- Files changed (paths only)
- Boundary violations fixed
- Any handoffs triggered
- Confirmation cargo test is green and warning-free
- Commit hash (`git rev-parse HEAD`) and confirmation it was pushed
