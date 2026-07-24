## Arena oversized spill strands the open chunk's free tail because only `chunks.last()` is consulted

category: perf | severity: low | verdict: CONFIRMED | finder: engine:interval-allen
outcome: fixed 537064d8

### Summary

`Arena::alloc` decides whether to open a new chunk by checking only the last chunk in the `chunks` vector. An oversized allocation (larger than `CHUNK_CAPACITY` = 64 KiB) pushes its own exactly-sized chunk, which becomes `last` and is completely full after the copy. The next ordinary allocation therefore opens a fresh 64 KiB chunk even when the chunk that was active before the oversized push still has nearly its whole capacity free — that free tail is never reachable again. The active-chunk identity is conflated with vector position; an explicit `active: usize` index that oversized allocations do not advance erases the waste, and index-based handles make the fix trivially sound.

### Evidence

- `crates/bumbledb/src/arena.rs:40-47` — the spill decision reads only `self.chunks.last()`:
  ```rust
  let needs_new_chunk = match self.chunks.last() {
      Some(chunk) => chunk.len() + bytes.len() > chunk.capacity(),
      None => true,
  };
  if needs_new_chunk {
      self.chunks
          .push(Vec::with_capacity(CHUNK_CAPACITY.max(bytes.len())));
  }
  ```
- `crates/bumbledb/src/arena.rs:48-51` — the write target is always `chunks.len() - 1`; after an oversized push, the prior partially-filled chunk is unreachable forever. `Vec::with_capacity` for `u8` yields exactly the requested capacity, so the oversized chunk is exactly full after `extend_from_slice` — no allocator slack rescues the next allocation.
- `crates/bumbledb/src/arena.rs:13-17` and module doc (lines 4-6) — `ArenaSlice` is a `(chunk, start, len)` index triple, explicitly designed so chunk storage may move; a separate `active` index invalidates no issued handle.
- Reachability of oversized facts: `crates/bumbledb/src/schema/validate.rs:61-68` gates only `derived_columns > u16::MAX` (65,535 word columns), and `crates/bumbledb-theory/src/type_desc.rs:58-67` gives every word column 8 bytes — so a legal relation with more than 8,192 word columns has a fact width above 64 KiB. No other fact-width gate exists in the schema or write path (verified by grep for fact-width checks; `FixedBytesWidthOutOfRange` at validate.rs:1413-1423 caps a single `bytes<N>` field at 64 bytes but not the fact).
- Single shared arena per transaction: `crates/bumbledb/src/storage/delta.rs:87` holds one `Arena`, and both alloc sites (`storage/delta/insert.rs:59`, `storage/delta/delete.rs:47`) route every relation's fact bytes through it, so wide and narrow facts interleave in the same chunk stream.
- Test coverage: `arena.rs:88-101` (`spills_into_new_chunks_without_moving_old_bytes`) exercises the oversized path for handle validity only; nothing pins or bounds the stranded space.
- Spec check: `docs/architecture/50-storage.md` (lines ~199, ~222) specifies only the arena discipline — fact bytes accumulate and free as a whole at commit/abort — and says nothing about chunk spill policy, so the current behavior is an implementation accident, not a documented trade.

### Bench impact

A transaction interleaving inserts of a wide relation (fact bytes > 64 KiB) with a narrow relation strands up to ~64 KiB of the previously active chunk per wide fact: every wide fact seals the arena's tail, and each following narrow fact opens a new 64 KiB chunk. The amplification is bounded (< 64 KiB per oversized allocation) and transient (the arena drops whole at commit or abort per 50-storage.md), which is why severity stays low — but it is systematic, and the representation can make it zero. No pathway to unbounded growth or correctness impact exists.

### Suggested fix

Track `active: usize` in `Arena`. Ordinary allocations route through `chunks[active]`; when a standard chunk fills, push a fresh 64 KiB chunk and advance `active` to it. An oversized request pushes its dedicated exactly-sized chunk without touching `active`, so the open chunk's free tail stays live. The branch lands only on the (already-branching) spill path, not the common bump path, and no issued `ArenaSlice` moves — consistent with the representation-first doctrine (`docs/design/representation-first.md`): the fix replaces a positional convention ("active = last") with an explicit one-word representation of the active chunk.
