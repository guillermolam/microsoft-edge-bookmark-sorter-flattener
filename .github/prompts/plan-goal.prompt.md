---
agent: "Plan"
---

Ultimate Target Artifact:
- The real Microsoft Edge bookmarks file must be transformed and validated:
  Windows: `C:\Users\guill\AppData\Local\Microsoft\Edge\User Data\Default\Bookmarks`
  WSL: `/mnt/c/Users/guill/AppData/Local/Microsoft/Edge/User Data/Default/Bookmarks`

Definition of Done (must all be true):
1) The tool produces a cleaned output that is functionally valid for Edge (JSON schema preserved, no data loss).
2) Flattening + merging invariants hold:
   - Folder uniqueness by case-insensitive name across the entire bookmark forest
   - No folder contains two subfolders with the same case-insensitive name (they must be merged)
   - Merge destination rule: “outermost wins” (canonical folder chosen by deterministic tie-breaker)
   - Cycles are detected and eliminated safely (graph becomes a DAG for processing; no infinite loops)
   - Empty folders are removed
3) URL dedup invariants hold:
   - Within any folder, only one bookmark per canonicalized URL
   - Same URL may exist in different folders
4) Determinism:
   - Running the tool twice on the same input produces identical output (byte-stable or JSON-stable canonical form)
5) Quality gates:
   - warnings-as-errors passes
   - tests pass
   - coverage >=98% for `src/**`
6) End-to-end validation on the real file:
   - Read the WSL path input
   - Produce an output file alongside it (never overwrite without backup)
   - Validate invariants by running the tool’s validate command (or add one if missing)
   - Optionally perform a dry-run diff summary for auditability

Safety rules:
- Never overwrite the original Bookmarks file without creating a timestamped backup in the same directory.
- Do not weaken tests to “match bugs”.
- Production fixes must be done only by the owning agent (rust-architect, determinism-ordering-agent, graph-scc-specialist, etc.).
- No serde or async in domain; no recursion.

Execution loop (repeat until Done):
1) #agent release-engineer-agent:
   - stablish canonical coverage + gates
   - report baseline coverage and lowest-covered modules

2) #agent cargo-test-agent (or test-fuzz-agent):
   - add deterministic unit/integration/property tests targeting lowest-covered modules
   - add an end-to-end test fixture that loads a sample Edge Bookmarks JSON (small) and checks invariants

3) If failures occur, route exactly one fix owner:
   - semantics/invariants unclear → #agent domain-guardian
   - implementation/architecture/warnings-as-errors → #agent rust-architect
   - ordering nondeterminism → #agent determinism-ordering-agent
   - SCC/cycle safety → #agent graph-scc-specialist
   - serde boundary/schema drift → #agent serde-compatibility-agent
   - async boundary/CLI → #agent tokio-cli-ergonomics-agent

4) Add real-file validation:
   - Ensure there is a CLI path like:
     - normalize --input <Bookmarks> --output <Bookmarks.cleaned.json> --backup
     - validate --input <Bookmarks.cleaned.json>
   - If missing, delegate:
     - CLI wiring → tokio-cli-ergonomics-agent
     - validation logic/invariants → domain-guardian + rust-architect

5) #agent release-engineer-agent:
   - re-measure coverage
   - confirm gates
   - confirm determinism (run twice comparison)
   - confirm end-to-end real-file validation succeeded

Stop only when all Definition of Done conditions are met.
