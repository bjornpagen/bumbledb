# PRD 12: Recursive Frame Executor

## Purpose

Remove hot-path cloning of bindings and source maps by replacing recursive immutable map cloning with explicit execution frames.

## Current Problem

The executor clones `BTreeMap<AtomOccurrenceId, ColtSource>` and `Binding` on recursive paths. This is simple but allocation-heavy and obscures the paper's nested-loop execution model.

## Required Design

- Replace binding `BTreeMap<usize, Vec<u8>>` with dense variable slots.
- Store fixed-width encoded values in reusable buffers or slot arrays.
- Replace source map cloning with frame-local source cursors or a mutable stack with undo records.
- Track frame pushes, pops, binding writes, binding conflicts, source replacements, and max depth.
- Keep the implementation understandable as the paper's recursive `join(all_tries, plan, tuple)`.

## Suggested Data Structures

```rust
struct BindingFrame {
    slots: Vec<Option<EncodedValueSlot>>,
}

struct SourceFrame {
    sources: Vec<ColtSource>,
    undo_log: Vec<SourceUndo>,
}
```

Exact shapes may differ, but hot-path clone counts must drop.

## Required Breaking Changes

- Delete `Binding` as a `BTreeMap` hot-path structure.
- Delete recursive source-map cloning in scalar and vectorized paths.
- Rewrite sink consumption to read from dense slots.

## Passing Criteria

- Trace counter for binding clones is zero or removed because cloning no longer exists.
- Trace counter for source-map clones is zero or removed because cloning no longer exists.
- Existing executor correctness tests pass.
- A stress test with duplicate witnesses still deduplicates public output.
- JOB q09 exact output remains unchanged.
- Global acceptance from PRD 00 passes.
