# Event storage, encoding and lifetime

Proposed implementation contract against `d76d31ab`. The important distinction
is **portable row content versus resident query tokens**. They are already
separate for text, and Event should follow that architecture.

## Owned value and resident representation

The initial carrier is a finite dense bitmap with complemented references.
A checked owned Event contains an immutable universe descriptor, an immutable
shared payload, and a polarity bit. No global manager is needed to construct
or retain one. Cloning may share payload ownership; it cannot share mutable
caller buffers. There are two payload cases: canonical zero, and nonzero words.

For an N-world universe, store `ceil(N / 64)` words in the nonzero case. Bit i
of the logical bitmap means world i. Choose canonical polarity from membership
of world zero: if it is one, invert valid membership bits before storing the
payload. The stored payload's bit zero is consequently always zero. Clear
unused bits in the final word. Canonical all-zero payloads use the zero case.

Decoding XORs each stored word with its valid-bit mask when polarity is one.
Complement flips polarity without changing the payload. Empty and full share
the zero case and require no N-bit allocation. A nonconstant result may still
require the entire bitmap; a tiny input list is not a promise of tiny storage.
For N=4,290 the nonzero word payload is 68 words, or 544 bytes, before metadata.
The u64 world count is an explicit public bound. A symbolic resident carrier
could help within that domain but would not make this dense portable encoding
cheap to export. A wider domain or different canonical wire form needs a
separate version/compatibility decision, not a transparent backend switch.

In query images, aim for two words: `(universe_token, payload_token_and_polarity)`.
Use ordinary two-word equality/sorting, not the interval-specific `WordPair`
comparison path. These tokens are valid only in the resolver generation that
minted them. All images, parameters, literals and computed stages in an execution
must resolve into a common generation before token comparison.

Extend the existing [generation resolver](../crates/bumbledb/src/work/cache.rs)
with a concrete Event table. Intern the complete descriptor and normalized
payload using exact content comparisons behind hash buckets. Collisions may
increase work but must never change equality. Tokens are not reused within a
live generation. Check token/polarity capacity before allocating an ID; do not
confuse exhaustion or a miss sentinel with a real Event.

Images and results pin the immutable payloads they reference. The resolver
must be able to reclaim unpinned payloads; merely pinning a generation must not
retain every Event ever seen. Resolve owners once per column/batch where possible;
release the resolver lock before algebra/classification scans and avoid an Arc
operation per word. Initial interning may compare immutable content under its
lock; measure that contention before designing a more complicated publication
protocol.
This follows the [text lifetime precedent](../crates/bumbledb/src/image/intern.rs)
without pretending its string comparator can classify regions.

Exact cross-generation equality compares content or explicitly re-interns it.
A prepared-query memo must include the existing source, relation-version and
generation identities. An Event parameter changed between runs must invalidate
its resolved value and any dependent result or predicate cache. A cache miss
cannot become “empty Event.”

## Durable rows: inline, without a new object store

The [canonical row codec](../crates/bumbledb/src/canonical.rs) already owns text
and bytes inline. Add Event there. LMDB rows, change sets and logical exports
carry the complete canonical value. No payload IDs, separate Event catalog,
reference counts, persistent summary indexes, orphan collection, or catalog
writes during reads are necessary.

This duplicates payload bytes when several durable facts contain the same
Event. That is an explicit tradeoff for using the established storage model.
Measure it, including canonical-row comparison and image decoding. Resident
sharing does not imply disk deduplication. Changing this later needs separate
storage evidence, not a hidden requirement in the type implementation.

Existing membership/determinant routing continues to narrow candidate buckets;
full canonical row/projection content decides equality. Event is not an eligible
bounded exact-scalar home. Pointwise determinant indexes route by the scalar
prefix, just as interval indexes do. Do not place a 544-byte payload in an LMDB
key or mistake its hash for its full value.

Inserts, deletes and insert/delete cancellation compare canonical content,
independent of arena allocation. A failed final-state judge publishes no row
changes. An aborted query may leave reclaimable resolver entries, but must not
publish results or mutate durable data. Snapshot readers continue observing their
own immutable row versions. Old results must remain usable after a writer
changes those rows or the originating query/cache is released.

## Proposed finite Event encoding

Pin this small format before implementation reaches persistence. It is distinct
from the historical BEVT graph/probability formats, which are not supported.
The proposed family bytes below have been checked against the baseline namespace;
S1's fixture gate freezes the actual encoding before any persistent publication.

| Field | Encoding |
| --- | --- |
| Family/version | Eight bytes `BDEVNT1\0` |
| Universe name | Sixteen exact UUID bytes |
| World count N | Big-endian u64, strictly positive |
| Polarity | One byte, exactly 0 or 1 |
| Payload kind | One byte: 0 for canonical zero, 1 for dense nonzero |
| Payload | None for kind 0; exactly `ceil(N / 64)` big-endian u64 words for kind 1 |

World `64*j+k` uses numeric bit k of word j. Endianness changes word encoding,
not that indexing rule. Dense payloads require bit zero clear, unused tail bits
clear, and at least one set bit. An all-zero dense spelling is rejected rather
than normalized on wire import. Ordinary constructors accept memberships in any
order, deduplicate them, and produce this normal form.

The header is 34 bytes; a nonconstant Coup Event occupies 578 payload bytes.
The canonical row adds its existing field tag/length framing: proposed field
tag 10, then big-endian u64 Event-byte length. The row's u16 arity is unchanged.
The schema type uses proposed tag 10 in its separate type-tag namespace.
All preexisting tags retain their bytes.

Parsing checks family, version, descriptor, flags, arithmetic, exact lengths,
canonicality and trailing bytes before publishing a checked value. A claimed
huge N with a short dense payload fails before allocation. Check ceiling division
and byte multiplication without `N + 63` overflow. Zero/full headers can describe
large finite universes without allocating dense payloads. Constructing a
nonconstant value over such a universe can fail allocation; never truncate it.

Encoding is a function of `(descriptor, members)` alone. Force independent
construction histories and hash collisions in tests. Exact bytes, not a digest
claim inside the payload, establish that two values are equal. Decoder and
encoder share the core contract; the log and native bridge do not invent codecs.

## Compatibility and transport

Preserve `bumbledb-schema-v6` and existing fingerprints when a schema has no
Event fields/literals. Append the unused Event type tag; do not globally rewrite
all existing schema identities. The proposed inline format does not change the
physical key layout, so keep core layout 7 if the S1 storage review confirms
that no existing bytes or interpretation change. If it cannot, stop and write
the precise compatibility change before modifying a layout number.

New binaries must open and operate on non-Event baseline stores unchanged.
Old binaries must refuse Event schemas/unknown fields before a write. Opening
with a different schema remains a fingerprint mismatch; adding an Event field
to an application schema uses the existing transition path, not an in-place
reinterpretation of stored bytes. No automatic migration from the discarded
implementation branch is included.

Support the normal transport routes: change-set parse/encode, snapshot export,
physical copy, logical backup/restore, history replay and schema transitions.
Physical copy needs no special Event traversal because the rows are self-contained.
Changing world meaning during a transition is caller work; copying the same
Event must preserve its bytes. Retry sealing must not generate new universe IDs.

The schema-file spelling is `event`; a JSON value can use
`{"event":"<lowercase canonical hex>"}` through the existing tagged-value grammar.
Pin parser/renderer symmetry and generated bindings. Query descriptions carry
ordinary Event literals/parameters, small expression nodes and literal masks;
never native pointers, captured solver objects or resolver tokens.

The existing log command-result record accepts scalars, not interval values.
Event remains outside that scalar-only record. This does not restrict Event
fields in database rows or query results. Do not expand the command-result
language merely because the shared `Value` enum gained a variant.

## Host and execution lifetime

The proposed host surface is deliberately small:

| Need | Rust shape | TypeScript shape |
| --- | --- | --- |
| Descriptor | Checked `EventUniverse { name, world_count }` | Checked `{ name: Uuid, worldCount: bigint }` |
| Constants | `Event::empty(scope)`, `Event::full(scope)` | `Event.empty(scope)`, `Event.full(scope)` |
| Supply members | `event::from_members(scope, indices, &work)` | `Event.fromMembers(scope, indices)` |
| Inspect | Descriptor, `is_empty`, `is_full`, `contains(index)` | Equivalent read-only value operations |
| Complement | Cheap immutable `event.complement()` | Cheap immutable complement operation |
| Binary construction | Core `event::{intersection, union, difference}` with `WorkContext` | Query expression builders; standalone numerical/model SDK not required |
| Transport | Checked core encode/decode with `WorkContext` | Owned `Event.fromBytes` / copied `toBytes` |

These are proposed spellings, not current callable APIs. `EventUniverse` is a
host descriptor, not another database field type or a registry. Constructors
reject out-of-range members, collapse duplicates and normalize order. Inspection
does not enumerate worlds. Host `Eq`/hash remain ordinary pure value operations;
core's potentially long scans use cancellable paths. Cheap operations and input
copying do not by themselves promise asynchronous execution.

Rust gets a checked immutable `Event` host value and borrowed result access,
with an explicit cheap owned clone when a caller retains it. The theory crate
owns value invariants; core owns resolver state and cancellable operations. Keep
that dependency direction. A small incremental checked word builder can validate
bounded chunks in theory while core checks `WorkContext` between chunks; this
needs no new solver-control trait or framework crate.

TypeScript gets an `event` field descriptor and a distinct owned Event value,
not an untyped byte blob interchangeable with `bytes<N>`. Its wrapper owns its
input; byte export returns a copy. World counts/indices use bigint. The native
boundary copies caller-owned buffers before queued work can observe mutation
and revalidates the complete canonical value. Follow current queue capture and
Effect error/cancellation conventions; no live native handles in JSON.

The S1 probe must settle how that checked value passes through the current
capture-before-worker code without retaining JS memory or creating two formats.
Do not claim all decode work is off the JavaScript thread without measuring and
checking the actual boundary. This is linear structural validation, not solver
admission.

`Answers`/`CompleteResult` must own or pin Event payloads independently of the
image resolver. Result paging limits delivery, not execution memory. A failed
page conversion must leave the delivery cursor unchanged. Repeated result use,
query teardown, database close and cache rotation need native lifetime tests.

## Resource honesty

[WorkContext](../crates/bumbledb/src/work.rs) has cancellation and allocation
errors, not a byte/work/time budget. Use checked size arithmetic, fallible
capacity growth and checkpoints during long scans, interning, codecs and group
folds. Existing process-wide OOM limitations remain; this proposal adds no hard
memory guarantee or new quota manager.

Likewise, aggregate scratch spill currently follows representation exhaustion,
not a byte budget. A scratch table containing only Event tokens would still pin
all bitmap payloads. For Event Pack, spill canonical group-union values using
the existing scratch facility, and release superseded group buffers when safe.
Spill tests must verify ownership as well as answers. They do not establish a
bound on the complete result, cached images or all process memory.
Event-valued group keys and dedup keys also need owners: a token in scratch does
not pin its payload. Retain the necessary key owners for the scratch lifetime
or store an owning canonical group header. Group-union spill must release
superseded accumulators without reclaiming values still referenced by such keys.
