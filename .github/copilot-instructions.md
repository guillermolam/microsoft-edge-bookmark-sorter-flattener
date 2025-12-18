# Copilot Instructions for Microsoft Edge Bookmark Sorter & Flattener

## Project Overview
- This is a **Rust-first, graph-aware normalization tool** for Microsoft Edge / Chrome bookmarks.
- The input JSON is **not treated as a tree**, but as a **directed graph** (folders can be reused by id/guid and may form cycles).
- The core goal is to **normalize, flatten, merge, deduplicate, and prune bookmarks deterministically** with **no data loss**.
- The system follows **DDD + event-driven orchestration**, with **async only at boundaries** (I/O, event streaming).

## Architectural Philosophy (must be preserved)
This project synthesizes:
- **Martin Fowler** — explicit architecture and boundaries
- **Uncle Bob** — SOLID, separation of concerns
- **Debasish Ghosh** — domain modeling over workflows
- **Martin Kleppmann** — data correctness, determinism, scalability
- **Graydon Hoare** — Rust idioms, ownership, minimal abstraction

Priority order:
1. **Correctness & Reliability** — no infinite loops, no data loss
2. **Performance & Scalability** — iterative O(V+E) algorithms
3. **Maintainability** — small modules, explicit invariants
4. **Extensibility** — strategies via traits *only when justified*
5. **Observability** — structured events and tracing

## High-Level Architecture
The codebase is organized using Clean / Hexagonal Architecture:

- `domain/`
  - Pure, synchronous logic only
  - Entities, value objects, invariants
  - No Tokio, no serde, no filesystem
- `usecase/`
  - Application orchestration
  - Emits events
  - Coordinates domain services
- `infrastructure/`
  - Adapters (serde JSON, filesystem, event bus)
  - Tokio usage is allowed here
- `interface/`
  - CLI entrypoints
  - Async runtime wiring
- `main.rs`
  - Thin bootstrap only

## Core Processing Model (important)
Copilot must respect this **mandatory pipeline**:

1. **Parse JSON → DTOs** (serde adapter)
2. **Build folder containment graph**
3. **Detect SCCs (iterative, no recursion)**
4. **Condense SCCs into a DAG**
5. **Determine canonical folder winners**
   - Global uniqueness by normalized folder name
   - Outermost winner rule
6. **Merge folders + URLs**
   - Per-folder URL dedup
   - Deterministic tie-breakers
7. **Prune empty folders**
8. **Emit deterministic JSON output**
9. **Emit events throughout processing**

Never assume the input is acyclic.

## Domain Rules (do not violate)
- **Folder identity**:
  - Unique globally by normalized name (case-insensitive, unicode-safe).
  - A folder may not contain two subfolders with the same normalized name.
- **Outermost merge rule**:
  - If the same folder name exists in multiple paths, all contents merge into the instance with the shortest root path.
- **URL deduplication**:
  - URLs deduplicated per folder only.
  - Same URL may exist in different folders.
- **Cycle safety**:
  - SCC detection is required before traversal.
- **No data loss**:
  - All removed/merged items must be recorded under `x_merge_meta`.

## Rust Style & Best Practices (strict)
- Prefer **structs + enums + functions** over Java-style services/managers.
- Prefer **static dispatch** (generics) over `dyn Trait` unless runtime selection is required.
- Avoid “Factory” or “Prototype” patterns unless they provide real value in Rust.
- Use **newtypes** (`NodeId`, `FolderKey`, `UrlKey`) instead of raw strings.
- Model node types as enums, not string comparisons.
- Minimize `pub`; expose small, intentional APIs.
- No recursion for traversal, SCC, or rebuild — use explicit stacks/queues.

## Async & Tokio Rules
- Async is allowed **only at boundaries**:
  - file I/O
  - event streaming
- Domain logic must remain synchronous.
- Do not spawn tasks per node.
- Use bounded `tokio::sync::mpsc` channels for events.
- If core computation is CPU-heavy, wrap it once in `spawn_blocking` (never scatter it).
- No nested runtimes.

## Serde Rules
- Serde DTOs live in `infrastructure/`.
- Domain types should not be polluted with serde attributes.
- Use:
  - `#[serde(tag = "type")]` enums
  - `#[serde(default)]` for optional fields
  - `#[serde(flatten)]` for unknown/forward-compatible fields
- Output ordering must be deterministic (explicit sorting).

## Patterns (Rust-adapted)
- **Strategy**: use traits or closures when behavior genuinely varies.
- **Observer**: event sink via `mpsc::Sender<Event>`, not observer lists.
- **Factory**: prefer `From` / `TryFrom` over factory objects.
- **Pipeline**: explicit, linear orchestration, not magical mediators.

## Tooling & Workflows
- Build: `cargo build`
- Run: `cargo run -- normalize`
- Test: `cargo test`
- Debug: `RUST_LOG=debug` + tracing
- Long tasks: run inside `tmux`

## What Copilot should NOT do
- Do not introduce recursion.
- Do not assume trees.
- Do not rely on HashMap iteration order.
- Do not add Java-style layers or empty abstractions.
- Do not mix domain logic with serde or Tokio.
- Do not invent new invariants without updating domain rules.

## Where to look for examples
- Domain invariants: `docs/domain-rules.md`
- Graph & SCC logic: domain graph modules
- Orchestration: `usecase/*`
- JSON boundaries: `infrastructure/serde_json_adapter.rs`
- CLI wiring: `interface/main.rs`
- Architectural rationale: `README.md`

---