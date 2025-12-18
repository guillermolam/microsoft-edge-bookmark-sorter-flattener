# Domain Rules (Authoritative Contract)

This document is the single source of truth for the bookmark normalizer’s invariants and semantics.

## Scope: input vs output

- **Input**: a Microsoft Edge / Chrome Bookmarks JSON document.
- **Output**: a deterministic, normalized Bookmarks JSON document.

The current `validate` command validates **post-normalization output invariants**. It is not intended to accept arbitrary real-world inputs before normalization.

## Definitions

### Folder name normalization (`FolderKey`)

A folder’s uniqueness key is its **normalized name**:

- `FolderKey = trim(name).to_lowercase()`

Notes:

- This is intentionally simple and deterministic.
- It is **not** full Unicode case-folding today.

### URL canonicalization (`UrlKey`)

Within a folder, URL bookmarks are deduplicated by a canonicalized URL string produced by the configured `UrlCanonicalizer` implementation.

## Folder invariants

### Global folder uniqueness

After normalization, folder names are globally unique across the entire forest by `FolderKey`.

### No duplicate subfolders

After normalization, within any folder, there must not exist two child folders with the same `FolderKey`.

### Outermost winner (“outermost wins”)

If the same folder name exists in multiple places, all contents merge into a canonical winner folder instance chosen deterministically by:

1. minimal path depth from any root
2. earliest `date_added`
3. smallest numeric `id` (if parseable)
4. smallest `guid` lexicographic

## URL invariants

### Per-folder URL deduplication

- Within any folder, there is at most one URL bookmark per canonicalized URL (`UrlKey`).
- The same URL may exist in different folders.

Winner selection is deterministic:

1. highest `visit_count`
2. latest `date_last_used`
3. earliest `date_added`
4. smallest `id`

## Empty folder pruning

After normalization:

- Root containers (e.g. `bookmark_bar`) are allowed to be empty.
- Non-root folders must not be empty.

## Provenance / no data loss (`x_merge_meta`)

Normalization does not silently discard merged information.

### Folder merge provenance

Winner folders record merge provenance under their `extra.x_merge_meta`, including (at minimum) merged names/paths/ids/guids.

### URL dedup provenance

Winner URL nodes record losers under `extra.x_merge_meta.merged_from` as a list of objects with fields like:

- `id`, `guid`, `name`, `url`, `path`

## Cycle safety (graph model)

Bookmarks JSON can represent graph-like structure in real exports (e.g. reused folder identities). The implementation guarantees termination by:

- using **iterative** algorithms (no recursion)
- computing SCCs for diagnostics/observability

At present, SCC results are emitted as events and recorded under top-level `x_merge_meta` for auditability; SCC condensation into a DAG is a planned evolution of the pipeline.

## Validation contract

`validate` checks that a document satisfies post-normalization invariants:

- global folder-name uniqueness (by `FolderKey`)
- no empty non-root folders
- no duplicate child folder names within a folder
- no duplicate canonicalized URLs within a folder
