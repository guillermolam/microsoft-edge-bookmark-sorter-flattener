---
agent: "Plan"
---

Continuously eliminate nondeterminism so that identical inputs produce identical outputs and tests/coverage/CI results are stable across runs.

Process loop (repeat until stable):
1) Ask release-engineer-agent to run the canonical build/test/coverage commands twice and report any diffs or instability signals (output diffs, ordering diffs, flaky failures).
2) Delegate to determinism-ordering-agent to:
   - identify nondeterministic iteration/ordering sources
   - define canonical ordering utilities and tie-breakers
   - add determinism tests
3) If nondeterminism is caused by:
   - SCC condensation ordering → delegate graph-scc-specialist
   - serde map ordering/roundtrip → delegate serde-compatibility-agent
   - async boundary concurrency effects → delegate tokio-cli-ergonomics-agent
4) Rerun cargo-test-agent to confirm.
5) Re-run step 1 until results are stable.

Rules:
- Do not “fix” determinism by removing data.
- Do not rely on HashMap iteration.
- All tie-breakers must be explicit, total, and stable.

Output:
- Determinism fixes applied (paths only)
- Tests added (paths only)
- Confirmation: 2 consecutive runs match
- Commit hash (`git rev-parse HEAD`) and confirmation it was pushed
