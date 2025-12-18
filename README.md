```markdown
# Microsoft Edge Bookmark Sorter & Flattener

A Rust application for cycle-safe normalization of Microsoft Edge / Chrome bookmarks JSON: detects cyclic folder references, collapses them into a DAG, globally merges folders by name, deduplicates URLs per folder, and prunes empty folders — deterministically, without losing data.

## Bookmark File Location
- Edge Bookmarks File: file:///C:/Users/guill/AppData/Local/Microsoft/Edge/User%20Data/Default/Bookmarks

---

## 🏗️ Architectural Philosophy

This codebase exemplifies the synthesis of Martin Fowler's enterprise patterns, Uncle Bob's SOLID principles, Debasish Ghosh's functional domain modeling, Martin Kleppmann's data-intensive application design, and Graydon Hoare's Rust philosophy. It prioritizes system quality attributes in this order:

1. Correctness & Reliability - No data loss, robust error handling
2. Performance & Scalability - Non-blocking, async-first design
3. Maintainability - Clean separation of concerns, SOLID principles
4. Extensibility - Strategy, Observer, and Factory patterns
5. Observability - Structured tracing and event-driven monitoring

A core realism check: bookmarks JSON looks like a tree, but in the wild it can behave like a graph (folders reused by id/guid), so correctness requires graph algorithms first.

---

## What the tool does

### Normalization goals
- Detect and neutralize cycles (no infinite loops)
- Flatten and merge folders globally by name (case-insensitive)
- Deduplicate URL bookmarks per folder (keyed by canonicalized URL)
- Remove empty folders created by merges/pruning
- Preserve provenance of merged/removed nodes in x_merge_meta

### Why graph processing is required
Some bookmark exports reuse the same folder node (by id/guid) in multiple places, even under descendants. That forms cycles like Folder A -> Folder B -> Folder A.

Traditional recursive "walk children" logic will loop forever or blow the stack. This project treats folder containment as a directed graph and makes it acyclic via SCC (Strongly Connected Components) condensation before any merging.

---

## Domain rules (the contract)

### Folder identity and merging
- A folder’s uniqueness key is its normalized name:
  normalize_folder_name(name) = unicode_casefold(trim(name)) (at minimum: trim + lowercase)
- Global invariant: only one folder per normalized name exists across all bookmarks.
- Local invariant: a folder cannot contain two subfolders with the same normalized name; they must merge.

### Outermost merge destination rule
If folder Z appears at multiple places, e.g.:
- X -> Y -> Z
- J -> Z
- K -> D -> Z

Then all Z contents merge into J -> Z and the other Z references are removed, yielding:
- X -> Y
- J -> Z (merged)
- K -> D

Winner selection is deterministic:
1. minimal path depth from any root
2. earliest date_added
3. smallest numeric id (if parseable)
4. smallest guid lexicographic

### URL deduplication per folder
- Node type "url" is deduped within each folder by canonicalized URL string.
- A URL may exist in multiple folders, but only once per folder.
- Winner selection:
  1) highest visit_count
  2) latest date_last_used
  3) earliest date_added
  4) smallest id

### Provenance and "no data loss"
Merges/dedups do not silently discard information. Removed duplicates contribute to metadata:
- folders: x_merge_meta.merged_names, x_merge_meta.merged_ids, x_merge_meta.merged_guids, optionally merged paths
- urls: x_merge_meta.merged_from with loser metadata

---

## The processing pipeline (SCC-first)

1. Parse JSON into DTOs (serde boundary)
2. Extract folder nodes and edges to build a directed graph
3. Compute SCCs (iterative, recursion-free)
4. Collapse SCCs into components and build a condensed DAG
5. Pick canonical folder instance per normalized name (outermost winner rule)
6. Merge folder children and URL nodes into winners
7. Prune empty folders created by the merge
8. Emit deterministic JSON output (same roots structure)
9. Emit events throughout for observability

---

## Patterns and principles (Rust-adapted)

This project uses classic patterns, implemented in a Rust-native way (type-driven, minimal ceremony).

### Strategy (pluggable algorithms)
Used where behavior genuinely varies:
- SCC detection algorithm choice (Tarjan/Kosaraju)
- URL canonicalization policy
- tie-break selection policies

Prefer generics/closures; use dyn Trait only when runtime selection is required.

### Observer (event-driven monitoring)
Event emission is designed for progress streaming and diagnostics:
- use a bounded tokio::sync::mpsc sender as an event sink
- subscribers run at the boundary (CLI/logging), not inside domain logic

### Factory (Rust-style construction)
Prefer From / TryFrom and small constructors over "factory objects".
Factories are only used if there are multiple representations that require explicit creation logic (e.g., DTO ↔ domain mapping).

---

## Async model (Tokio best practices)

Async is used at boundaries:
- reading/writing bookmark JSON
- streaming events (bounded channels with backpressure)

Core graph/merge logic stays synchronous and deterministic. If the core compute step becomes CPU-heavy, wrap it once in tokio::task::spawn_blocking to avoid stalling the runtime.

---

## Serde model (compatibility-first)

Serde is confined to infrastructure DTOs:
- DTOs use serde(tag = "type") for node variants
- serde(default) for missing fields
- serde(flatten) extra to preserve unknown fields for forward compatibility

Domain types avoid serde attributes unless the type is inherently persisted.

Output ordering is explicitly sorted to guarantee determinism.

---

## Repository structure (DDD + Clean Architecture)

- src/domain/
  Entities, value objects, invariants, domain events, domain services (traits).
  No Tokio. No serde. No filesystem.

- src/usecase/
  Orchestrates the normalization pipeline and emits application events.

- src/infrastructure/
  Serde DTO adapters, IO adapters, event bus implementation, utilities.

- src/interface/
  CLI wiring, argument parsing, event streaming output.

---

## Usage

Build:
    cargo build

Run (example):
    cargo run -- normalize --in /path/to/Bookmarks --out /path/to/Bookmarks.normalized

Emit NDJSON events (example):
    cargo run -- normalize --in /path/to/Bookmarks --out /path/to/Bookmarks.normalized --emit-events

Test:
    cargo test

---

## Observability

- tracing spans annotate phases: parse, graph_build, scc, merge, prune, emit
- event stream can be printed or written as NDJSON for tooling

---

## Why this isn’t Java

The architecture borrows proven ideas (ports/adapters, strategies, eventing), but stays Rust-native:
- enums over stringly-typed dispatch
- newtypes for identity and keys
- minimal pub, small modules, explicit ownership
- no "Manager/Service/Impl" ladders
- deterministic sorting instead of relying on map iteration order

The result: fewer moving parts, clearer invariants, better performance, and less accidental complexity.

---

## Key Takeaways

1. Bookmarks “trees” can hide cycles — treat them as graphs.
2. SCC condensation turns cyclic structures into a DAG you can safely process.
3. Global folder merging by normalized name is a domain invariant, not a heuristic.
4. Deterministic tie-break rules make results auditable and repeatable.
5. Async belongs at boundaries; correctness lives in the domain.
```
