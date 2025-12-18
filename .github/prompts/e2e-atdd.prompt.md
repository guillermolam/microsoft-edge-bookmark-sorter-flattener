---
agent: "Plan"
name: "Configure Cucumber ATDD E2E Suite"
description: "Set up cucumber-rs for ATDD end-to-end testing of the Edge bookmark normalizer, covering flattening, dedup, cycle-safety, determinism, backups, and CLI validation goals."
tools:
  [
    "agent",
    "read/problems",
    "search/changes",
    "execute/testFailure",
    "search/usages",
  ]
---

Objective
Configure https://github.com/cucumber-rs/cucumber for this Rust workspace and generate production-grade ATDD E2E tests (Gherkin + step defs) that validate the end-to-end CLI behavior and all invariants.

Ultimate Target Artifact (real file)

- Windows: `C:\Users\guill\AppData\Local\Microsoft\Edge\User Data\Default\Bookmarks`
- WSL: `/mnt/c/Users/guill/AppData/Local/Microsoft/Edge/User Data/Default/Bookmarks`

Global E2E Definition of Done (must all be testable)

1. CLI safety and IO:
   - Tool reads bookmarks JSON from an input path
   - Tool writes cleaned output to an output path
   - Tool never overwrites input without a timestamped backup (backup enabled by flag)
   - Tool supports dry-run/report mode (no file modifications) and emits a summary
   - Tool supports validate mode/subcommand that checks invariants on a file
2. Folder invariants:
   - Folder uniqueness across entire forest by case-insensitive name
   - No folder contains two subfolders with same case-insensitive name (must merge)
   - Merge destination rule “outermost wins” with deterministic tie-breakers
   - Empty folders removed
3. URL invariants:
   - Within each folder, only one bookmark per canonicalized URL
   - Same URL may appear in different folders
4. Cycle safety:
   - Cycles are detected and handled; normalization terminates; no infinite loops
5. Determinism:
   - Two consecutive runs on identical input produce identical output (stable ordering + stable JSON form)
6. Quality gates alignment:
   - E2E suite runs under CI with warnings-as-errors and remains deterministic

Approach

- Use cucumber-rs for ATDD with Gherkin features under tests/features/\*\*
- Implement step definitions in Rust under tests/cucumber/\*\*
- Execute the compiled CLI binary via std::process::Command in step defs (true E2E)
- Use temp directories for fixtures and outputs; never touch the real Edge file in CI
- Provide fixtures representing:
  - duplicates within folder
  - same URL in different folders
  - folder name collisions (case-insensitive) at multiple depths
  - cycle/reference-like structures requiring cycle handling
  - empty folders
  - large-ish fixture for performance smoke
- Ensure expected outputs are checked via invariant validation, not fragile full-file snapshots unless canonical JSON is guaranteed

Non-negotiables

- No recursion in test harness that could mask stack issues; keep tests iterative
- Determinism: tests must not depend on filesystem ordering or HashMap iteration
- Avoid embedding production code changes in the test agent; if tests reveal missing CLI/validate/report functionality, route to tokio-cli-ergonomics-agent or rust-architect as appropriate

Execution Plan (delegate by ownership)

1. #agent release-engineer-agent
   - Add/confirm dev dependencies and runner setup for cucumber
   - Ensure CI can run cucumber suite deterministically
   - Decide canonical commands:
     - cargo test (unit/integration)
     - cargo test -p <e2e crate> or cargo test --test cucumber_e2e
2. #agent tokio-cli-ergonomics-agent
   - Ensure CLI supports: normalize --input --output --backup, validate --input, dry-run/report
   - Ensure exit codes: 0 success, non-zero on invariant violations/parse errors
   - Ensure stable JSON output format and consistent diagnostics
3. #agent serde-compatibility-agent
   - Provide safe fixture parsing and forward-compatible expectations
   - Confirm DTO boundaries (tests should interact only via CLI + JSON files)
4. #agent domain-guardian
   - Translate invariants into explicit Gherkin scenarios (Given/When/Then) with deterministic tie-break expectations
5. #agent determinism-ordering-agent
   - Define what “identical output” means for tests (byte-identical vs canonicalized compare)
   - Add a deterministic comparator approach for E2E assertions
6. #agent graph-scc-specialist
   - Provide cycle-focused scenarios and the minimal fixtures that reproduce cycle edge cases safely
7. #agent test-fuzz-agent
   - Implement the cucumber harness:
     - feature files
     - step definitions
     - fixture management
     - running the CLI binary
     - assertions for report + validate output
   - Run the suite and provide a Failure Brief if anything fails
8. #agent rust-architect (only if needed)
   - If tests expose missing behavior (validate/report hooks, stable output, invariant enforcement), implement fixes without changing intended semantics
9. #agent release-engineer-agent (final)
   - Wire cucumber into CI and local dev workflow
   - Confirm: E2E suite passes, deterministic, and contributes meaningfully to coverage (without inflating artificially)

Required Deliverables (files/structure)

- Cargo.toml updates (workspace) for cucumber dependencies and an E2E test crate/target
- tests/features/\*.feature with scenarios covering all invariants
- tests/cucumber/ (or tests/e2e/) with step defs
- A minimal runner test (e.g., tests/cucumber_e2e.rs) that integrates cucumber-rs
- Fixture JSON files under tests/fixtures/\*\*
- Documentation: docs/atdd.md describing how to run E2E suite locally and in CI

Acceptance Criteria (must be proven by E2E)

- Each invariant has at least one scenario that fails before normalization and passes after
- CLI emits report summary containing counts for:
  - merges performed
  - duplicates removed (folder + url)
  - cycles detected/broken
  - empty folders removed
- Determinism scenario:
  - run normalize twice on same fixture => outputs compare equal by agreed comparator
- Safety scenario:
  - normalize with --backup produces a timestamped backup and does not overwrite the original without it
- Validation scenario:
  - validate returns non-zero for a deliberately invalid fixture and zero for normalized output

Start now

- Begin with #agent release-engineer-agent to add cucumber-rs and define canonical commands and test target structure.
