# PRD 03: COLT Source Handle Cutover

## Purpose

Replace heap-owning COLT source handles with compact arena source handles.

## Required Design

The runtime must carry source handles, not node ownership:

```rust
#[derive(Clone, Copy)]
struct ColtSource {
    arena_id: u32,
    node_id: ColtNodeId,
    vars_id: SchemaVarsId,
}
```

Exact names may differ. The core requirement is that cloning a source is copying integers only.

## Required Work

- Introduce a query-local owner for arenas and base images, for example `QuerySources` or `SourceStore`.
- Make `source_for` return a compact handle.
- Update source frame undo records to store compact handles.
- Keep current legacy COLT implementation available until behavior is equivalent.

## Passing Criteria

- Source handle size is asserted in a unit test and remains at most 24 bytes.
- Source replacement undo no longer clones `Rc` handles in the new path.
- Existing executor tests pass with the new handle path enabled for at least one fixture.
- No public API changes.
- Global gates pass.
