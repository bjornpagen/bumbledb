# Q2 consumer audit during ordinary builds

Source audit only; no additional capture or engine edit. The fixed 18-case
Q2-vs-Q1 panel is already frozen and is not changed by this note.

## Three production consumers, not just two sinks

1. `exec/sink.rs`: `SpillSet.ram: WordMap<()>`. Full-key seen sets,
   insertion-ordered results, recursive delta iteration, representation-bound
   spilling. The numeric saved controls cover these, including repeat/release.
2. `exec/sink.rs`: `GroupTable::Hashed(WordMap<usize>)`.
   `aggregate/groups.rs::probe_group` inserts exactly the previous map length
   as the new group index and never modifies a stored index. Shared
   `aggregate/spill.rs::for_each_ram_group` consumes the map's keys and indices
   for both resident finalization and spilling. The numeric controls cover
   hashed and exact-dense groups, DNF/written union and ordinary fold results.
3. `api/prepared.rs`: `ResolveMemo.ranges: WordMap<(u32,u32)>`.
   This maps an intern word to its `(start,len)` in the current answer text
   heap. Unlike aggregate group indices, those values are real payload and
   are mutated after the initial insert. Do NOT delete generic value support
   on the mistaken assumption that all production values are row ordinals.
   The module is crate-internal, so no public SDK change is needed for a
   representation experiment, but all three consumers still matter.

`ResolveMemo::resolve_checked` first inserts `(0,0)`, resolves borrowed text
into the caller's answer heap, checks u32 start/length bounds, then fills the
slot. Resolution errors clear the memo, so the placeholder is never reused
as a successful hit. Its `last` cache bypasses the hash table for repeated
adjacent words. A text performance control must include shuffled/repeated
tokens as well as one coherent repeated token; the latter alone hides probes.
Binding/execution resets clear memo state because ranges belong to one answer
heap. Caller-owned answers must survive prepared release and later executions.

## Existing evidence and the remaining coverage gap

Q2's 1,327/1,340 library gates include
`finalize_materializes_each_distinct_intern_once`: 64 equal strings produce
one memo entry and one copy, then 16 different strings survive changed binds
and repeated execution. Ordinary query verification covers typed results too.
This is meaningful semantic coverage, not a text speed/allocation comparison.

The 41 owner/counter cases observe sinks and answers with numeric SQL goldens.
Their "all maps" means every recorded sink map, NOT the unobserved text memo.
The fixed ordinary 18-case panel is numeric as well. The stock family named
`string` binds an Instrument.symbol but returns Posting.id/amount: it also
does not exercise text-output memoization. Name-based coverage is insufficient.

Before accepting Q2 generally, add a small separately declared saved-corpus
text-output control. Holder.name, Instrument.symbol and Org.name already
exist in the frozen Ledger/SQLite corpus, so no new corpus/full trace is
needed. Use independent exact SQL strings, repeated nonadjacent symbols from
a Posting/Instrument join and a distinct-name scan, present/empty bounds,
changed binds, warm repeat, release/refill and retained old answers. Inspect
memo owners and answer heap separately after counters stop; do not equate
text bytes with map backing. Include cold and warm ordinary timing when the
numeric panel supports continuing this representation. Check failed text
resolution/reset semantics through the existing direct correctness fixtures.

## Possible later simplification, not a selected change

Q2 exposes dense row ordinals internally. Hashed aggregate values currently
duplicate those ordinals in an additional Vec<usize>: observed Q2
groups-8192 retains 65,536 value bytes; groups-100000 retains 1,048,576.
A shared internal ordinal-returning entry could allow that consumer to use
WordMap<()> and enumerate group keys in insertion order, while retaining
generic payloads for ResolveMemo. This may remove a dependent value load and
one owner; it is not a speed claim or a reason to alter the running Q2 build.
Prove index alignment with accumulator/count/Pack arrays across clear,
reaim, growth and spill before testing that follow-up. Keep dense radix
tables and unrelated P2 row-input experiments separate. Do not add a second
hash implementation merely to avoid the generic value Vec in one consumer.
