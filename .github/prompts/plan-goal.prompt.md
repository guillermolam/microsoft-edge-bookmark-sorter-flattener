---
description: Plan the work needed to meet the Definition of Done for this repo.
name: plan-goal
argument-hint: "task=<what are we trying to accomplish?>"
agent: "Plan"
tools:
   - agent
   - read/problems
   - search/changes
---
# Prompt files (VS Code) — usage + best practices

This file is a VS Code Copilot **prompt file** (`.prompt.md`) stored in `.github/prompts/` so it can be run on-demand from Chat.

How to run:
- In VS Code Chat, type `/plan-goal task="<your task>"` (or type `#prompt:` and pick this file).

Command examples (copy/paste):
- `/plan-goal task="Make Trunk pass: ignore trailing whitespace, disable markdownlint, keep diffs minimal"`
- `/plan-goal task="Add deterministic tests for SCC cycle handling and verify byte-stable output"`

Best practices (apply them when generating the plan):
- Be explicit about the goal and the expected output format (a numbered todo list with clear verification steps).
- Prefer referencing existing workspace files and instructions over duplicating them (link them).
- Use concrete commands for verification (build/test/lint) and state what success looks like.
- Use `${workspaceFolder}` and `${input:...}` variables instead of hardcoding paths when possible.

Key repo guidance (reference, don’t restate):
- Architecture + constraints: [../copilot-instructions.md](../copilot-instructions.md)
- High-level rationale + commands: [../../README.md](../../README.md)

Input:
- Task: ${input:task:What should the plan accomplish?}

Output format (required):
- A short assumptions block (1-3 bullets)
- A numbered plan with 3–8 steps
- Each step includes: what to do, where (file paths/symbols), and how to verify (exact commands)

When executing in Agent mode:
- Use #tool:search/changes to confirm only intended files changed.
- Use #tool:read/problems after build/test to capture compiler/test failures.

# Commands (copy/paste)

Repo + quality gates:
- `cd ${workspaceFolder}`
- `cargo build`
- `cargo test`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`

Optional coverage gate (if enabled for the task):
- `cargo install cargo-tarpaulin`
- `cargo tarpaulin --workspace --all-features --out Json --output-dir target/tarpaulin --fail-under 98`

CLI (examples):
- Normalize: `cargo run -- bookmarks normalize --in <input.json> --out <output.json> [--emit-events] [--backup]`
- Validate: `cargo run -- bookmarks validate --in <input.json>`

Optional real-file end-to-end (WSL, safe):
- `IN='/mnt/c/Users/<you>/AppData/Local/Microsoft/Edge/User Data/Default/Bookmarks'`
- `ts=$(date +%Y%m%d-%H%M%S)`
- `cp -a "$IN" "$IN.bak.$ts"`
- `OUT="$IN.cleaned.$ts.json"`
- `cargo run -- bookmarks normalize --in "$IN" --out "$OUT" --emit-events > "$OUT.events.ndjson"`
- `cargo run -- bookmarks validate --in "$OUT"`

Optional determinism check (same input => identical output bytes):
- `OUT1="$IN.cleaned.$ts.1.json"; OUT2="$IN.cleaned.$ts.2.json"`
- `cargo run -- bookmarks normalize --in "$IN" --out "$OUT1"`
- `cargo run -- bookmarks normalize --in "$IN" --out "$OUT2"`
- `cmp -s "$OUT1" "$OUT2"`

# Ultimate Target Artifact:

- The real Microsoft Edge bookmarks file must be transformed and validated:
   Windows: `C:\Users\<you>\AppData\Local\Microsoft\Edge\User Data\Default\Bookmarks`
   WSL: `/mnt/c/Users/<you>/AppData/Local/Microsoft/Edge/User Data/Default/Bookmarks`

# Definition of Done (must all be true):

1. The tool produces a cleaned output that is functionally valid for Edge (JSON schema preserved, no data loss).
2. Flattening + merging invariants hold:
   - Folder uniqueness by case-insensitive name across the entire bookmark forest
   - No folder contains two subfolders with the same case-insensitive name (they must be merged)
   - Merge destination rule: “outermost wins” (canonical folder chosen by deterministic tie-breaker)
   - Cycles are detected and eliminated safely (graph becomes a DAG for processing; no infinite loops)
   - Empty folders are removed
3. URL dedup invariants hold:
   - Within any folder, only one bookmark per canonicalized URL
   - Same URL may exist in different folders
4. Determinism:
   - Running the tool twice on the same input produces identical output (byte-stable or JSON-stable canonical form)
5. Quality gates:
   - warnings-as-errors passes
   - tests pass
   - coverage >=98% for `src/**`
6. End-to-end validation on the real file:
   - Read the WSL path input
   - Produce an output file alongside it (never overwrite without backup)
   - Validate invariants by running the tool’s validate command (or add one if missing)
   - Optionally perform a dry-run diff summary for auditability

# Safety rules:

- Never overwrite the original Bookmarks file without creating a timestamped backup in the same directory.
- Do not weaken tests to “match bugs”.
- Production fixes must be done only by the owning agent (rust-architect, determinism-ordering-agent, graph-scc-specialist, etc.).
- No serde or async in domain; no recursion.

# Execution loop (repeat until Done):

1. #agent release-engineer-agent:
   - establish canonical coverage + gates
   - report baseline coverage and lowest-covered modules

2. #agent cargo-test-agent (or test-fuzz-agent):
   - add deterministic unit/integration/property tests targeting lowest-covered modules
   - add an end-to-end test fixture that loads a sample Edge Bookmarks JSON (small) and checks invariants

3. If failures occur, route exactly one fix owner:
   - semantics/invariants unclear → #agent domain-guardian
   - implementation/architecture/warnings-as-errors → #agent rust-architect
   - ordering nondeterminism → #agent determinism-ordering-agent
   - SCC/cycle safety → #agent graph-scc-specialist
   - serde boundary/schema drift → #agent serde-compatibility-agent
   - async boundary/CLI → #agent tokio-cli-ergonomics-agent

4. Add real-file validation:
   - Ensure there is a CLI path like:
     - normalize --input <Bookmarks> --output <Bookmarks.cleaned.json> --backup
     - validate --input <Bookmarks.cleaned.json>
   - If missing, delegate:
     - CLI wiring → tokio-cli-ergonomics-agent
     - validation logic/invariants → domain-guardian + rust-architect

5. #agent release-engineer-agent:
   - re-measure coverage
   - confirm gates
   - confirm determinism (run twice comparison)
   - confirm end-to-end real-file validation succeeded

Stop only when all Definition of Done conditions are met.
