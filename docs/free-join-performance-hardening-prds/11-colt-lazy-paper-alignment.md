# PRD 11: COLT Lazy Paper Alignment

## Purpose

Make COLT behave like the paper's Column-Oriented Lazy Trie while preserving Bumbledb set semantics and LMDB snapshot adaptation.

## Required Design

- COLT leaves store offsets into base-image columns, not tuple bytes.
- COLT map nodes are created only on `get` or when iteration over keys is required.
- Iterating a suffix vector must stream tuples directly from column buffers without forcing a map.
- Dynamic cover key-count requests must not force maps unless explicitly requested by policy.
- Source filters must shrink offset lists before deeper trie work.
- COLT counters must expose nodes created, nodes forced, offsets scanned, map entries, tuple yields, get calls, and misses.

## Required Map Policy

- Use performance-oriented maps internally unless deterministic ordering is required by a test that cannot be moved to final result sorting.
- Public output determinism must come from `QueryResultSet` canonical sorting, not COLT map iteration order.
- If `HashMap` replaces `BTreeMap`, tests must not assume internal iteration order.

## Required Breaking Changes

- Remove test expectations that require COLT keys in sorted map order unless they explicitly sort collected keys.
- Replace `Rc<RefCell<...>>` if traces show borrow/refcell overhead is material. If not replaced here, record it as an open trace-backed optimization target.
- Remove redundant `vars` clones where a source can borrow or use interned schema metadata.

## Passing Criteria

- A test proves a cover-only suffix iteration does not force a map.
- A test proves `get` forces only the node needed.
- A test proves dynamic key-count estimation does not force by default.
- Trace distinguishes COLT construction from COLT force.
- JOB q09 exact output remains unchanged.
- Global acceptance from PRD 00 passes.
