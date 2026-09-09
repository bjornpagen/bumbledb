# Q2: dense payloads behind the existing hash probe

An isolated experiment on frozen Q1, not an accepted change or a combination
with the other candidates. No full trace/suite, commit, push or release.

## Evidence

q1-growth-model-1 derives monotonic first-fill work from recorded owners and
the unchanged doubling/load law. Large seen maps rehash 174,752 entries from
empty vs 131,071 with the former hint: +43,681 visits, +87,362 copied key words
and +2,359,250 requested table bytes. The complete computed/union allocation
deltas differ only by -16/-32 bytes of removed preparation metadata. Hashed
groups at 8,192 add 5,453 rehash visits/43,624 value-copy bytes. The DNF-500
seen+group model adds 2,137,978 bytes vs measured 2,137,914 (-64 metadata).
Intermediate work is source-derived, not a new event/CPU capture.

Ordinary Q1 combined medians regress 16.51% on DNF-500 and 4.52% on union-100000,
unflagged. Warm groups-8192 also loses; growth alone cannot explain that.
The actual 100,000-row two-word map holds 9,961,479 bytes at 524,288 slots.
Its keys and values live at sparse slots, with a dense list of those slots.
Growth allocates new sparse payload and copies all live payload while rehashing.

## New representation and required controls

Invert the mapping: keep keys and Copy values packed in insertion order;
hash slots hold u32 row ordinals. Keep ctrl SWAR probing, generation stamps,
load factor and demand-driven doubling. Growth rebuilds only the index,
without moving/reallocating payload. All value access becomes safe, removing
MaybeUninit/unsafe from WordMap. The extra ordinal load on live matches and
tiny/zero-width index costs must be measured, not waved away.

Preserve exact duplicates, accumulated values, order, iter_since clipping and
cloning, zero width, widths 1–8 and dynamic width. Consult ordinals only after
current-generation ctrl validation, never for cleared payload. Cover mirrored
tail, disjoint-key saturation, 300+ resets, warm reuse and release/refill.
Duplicates at the load boundary must not allocate. Allocate replacement index
arrays before mutation; rehash must not touch payload ownership. Reserve both
payload vectors before publishing, and preserve contents across constructor
panic even after index growth. Keep prior arithmetic/representability bounds,
scalar/bulk sharing, cancellation, spill, recursion, Pack and answer ownership.
No allocator/quota/fallback/API/disk change; exact dense-group tables unchanged.

## Gates

Keep semantic tests while updating their physical slot-to-row assertions. Add
dense-owner, no-payload-copy-on-grow, stale-ordinal and panic regressions. Run
focused tests, strict lint, ordinary/allocation libraries and independent oracle.
Use fresh Cargo target AND build directories and verify executable identities.
Then measure exact owners/requests and ordinary controls against Q1, including
large DNF/union, groups-8192, range, point and tiny maps. Preserve all adverse
samples. No performance acceptance from memory reduction or model projections.

## Correctness checkpoint, September 9

Gate 1 is preserved as failed: four unchecked u64-to-usize conversions in the
new reuse test failed strict lint. Keep the row count as usize and convert
only bounded test keys to u64; no engine behavior changed for this repair.

Gate 2 finishes 11:47:20 UTC with exact candidate fingerprint
`dc78c0d7123b10edc7f9df4751a9d3a4792b65e4a428458f639c8a9d6e936f3c`.
Format, 26 WordMap tests (one intentionally ignored), 67 sink tests, strict
engine/benchmark all-target lint, 1,327 ordinary and 1,340 allocation-enabled
library tests (18 intentionally ignored each), ordinary release build and
all 2,879 independent query-oracle cases pass. Fresh release executable SHA:
`1ddc0174e8f7fb6af7d3318eb662a21bb63d8af67e4e5fdf9ab463f53e7483f1`.
Both Cargo caches were isolated and the engine and executable artifacts are
fresh. Q1 and all six prior candidate identities are unchanged.

The matched allocation controls explicitly redefine the fifth owner: Q1's
dense list of sparse slots becomes Q2's sparse index of dense row ordinals.
The shared controls and independent SQL answers remain unchanged. Count all
requested bytes, not just final retained backing, and reconcile their net
difference against every observed map. A common-field discriminator is
compiled unchanged against Q1/Q2 to require baseline failure and candidate
success. These are uninstrumented-for-CPU count controls, not a full trace.

Before observing Q2 memory or speed results, `q2_time_experiment.py` fixes an
18-case ordinary Q2-vs-Q1 panel. It includes all major Q1 adverse controls,
empty/tiny/computed/recursive maps, triangle, two range draws, present/absent
points and dense groups. The driver is the frozen Q1 ordinary driver: ABBA,
16 fresh prepare/first/combined/second samples and 320 per warm mode, all
answers checked. No retry/filter/normalization or full benchmark is allowed.
This checkpoint is not allocation or performance acceptance.
