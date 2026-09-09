# Q3: one dense row cardinality and ordinal-first entries

An isolated follow-up to Q2, selected from Q2-INSERTION-CODE-REVIEW.md and
the unflagged warm losses in Q2-ORDINARY-TIMING-REVIEW.md. No new full trace,
full benchmark, allocator scheme, SDK or disk-format change.

Q3 began as a byte-exact copy of frozen Q2 in q3-source. Q2, Q1 and six prior
candidate identities were verified before editing. Only WordMap implementation
and its tests differ from Q2; primary worktree changes are untouched.

The implementation deletes WordMap.len in favor of values.len, shared by
real values, zero-sized values and zero-width keys. Internal entries return
(row, inserted). Only get_or_insert_with borrows a value; set and bulk callers
discard an ordinal, not a mutable reference with an unnecessary bounds check.
Value capacity is reserved before Copy-key extension, removing the redundant
explicit key reservation. Index publication remains last. Probe loops, load
factor, generation/stamp rules, payload/index capacities, spills and all three
production consumer interfaces are unchanged.

The new tests exercise ordinals and mutable values across all fixed widths,
dynamic width, growth and clear; a side-effecting ZST Default panics on the
third new key after growth, checking the exact published prefix, retry and
duplicate no-construction behavior. Existing tests retain constructor-panic,
600-reset, pointer-stable rehash, order, dynamic width and scalar/bulk coverage.
The representation-bound test uses vec![(); count], the standard library's
constant-work unit specialization. Generic resize would loop in a debug build;
source inspection caught that before running the test.

## Gates

q3-gates-1 is preserved as failed: the new ordinal test was in a sibling
module and could not access the private entry method (E0624). No test ran.
The method is now pub(super), visible only inside WordMap, matching the
existing probe/entry_dyn helper scope. No public library API is exposed.
Gate 2 uses fresh, separate Cargo target AND build directories.

No Q3 allocation or ordinary timing panel has run. Correctness success alone
will not establish performance acceptance. First inspect the freshly built
ordinary scalar and bulk code for actual removal of the redundant checks and
stores. Do not infer speed from source or function size. Then price affected
warm queries plus negative controls and close ResolveMemo text-output coverage
using the saved corpus. Keep the earlier losses and every clock flag.

## Gate 2 complete, September 9 12:16:17 UTC

Source fingerprint:
`6bc03e29d2e426a50995b5945efe2d1761966d816601f06a046edb183d39ff3b`.
28 focused WordMap tests pass (one intentionally ignored), 67 sink tests,
strict engine/benchmark lint, 1,329 ordinary library and 1,342 allocation-enabled
library tests pass (18 intentionally ignored each). Fresh optimized ordinary
release build and all 2,879 independent oracle cases pass. Executable SHA:
`f1899d6607176e3c2968c9955a34cccda0b975a5fafa3ee0ac89cb6f5c5a608a`.
Q2, Q1 and six prior identities remain unchanged. No ordinary timing or
matched allocation-owner panel was run for Q3. The allocation-enabled unit
suite is not a substitute for the owner comparison or text consumer controls.

## Generated-code checkpoint

q3-codegen-1 exports 21 selected functions from the actual Q2/Q3 ordinary
gate executables. These are not the earlier q1-timing driver binaries; their
source paths and containing code differ. No cycle, instruction-cache or
elapsed-time gain follows from their static sizes.

The intended hot-path deletions did occur for width two:

- Scalar (q3-function-4.log) dispatches through 0x10011bb10 to normal probe
  0x10011d058. Key equality at 0x10011d12c/130 jumps directly to false return
  0x10011b2bc. The discarded-value bounds check is gone. Vacant insertion
  0x10011d178 retains only the value-capacity precheck before branching with
  width TWO (0x10011d1b0) to the copy helper call 0x10011d890. The earlier
  duplicate key-reserve check is gone; the out-of-line copy is NOT gone.
  Publication increments values.len once at 0x10011d8a4/8a8, then publishes
  slot/stale/ctrl/stamp and returns true without a second row-count update or
  unused returned-reference check.
- Bulk (q3-function-6.log) dispatches to 0x10011f178. Both normal and boundary
  duplicate hits jump directly to next row 0x10011f16c (0x10011f368/418), with
  no value-reference bounds check. Vacant insertion has ONE key-capacity test
  at 0x10011f490 before its inlined 16-byte copy at 0x10011f49c/4a0. A single
  values.len update at 0x10011f4b8/4bc replaces the two counts. Ctrl/stamp
  publication returns straight to next row at 0x10011f518/530.

ZST overflow checks before/after key copy remain in BOTH paths. Safe key,
index, ctrl and stamp bounds remain. Do not report all checks eliminated.

Hot function sizes Q2 -> Q3: scalar hashed insertion 11,220 -> 10,284 bytes;
bulk 11,256 -> 10,360; group probe 11,496 -> 10,596; text resolve 10,756 ->
10,684. Proved-unique insertion remains 512. However Q3 emits separate grow
bodies for WordMap<()> (9,172) and WordMap<usize> (9,180), where Q2 shared one
9,180-byte body. Scratch insertion also changes 1,000 -> 984 in these gate
binaries. Total executable file size is 7,305,408 -> 7,305,536 bytes (+128),
NOT an overall executable-size reduction. This code-layout change belongs in
the timing interpretation; don't add another refactor merely to merge symbols.

NEXT: freeze Q3 and measure its actual allocation owners/request tuples, then
ordinary targeted affected/negative controls using the SAME comparison path
and timing driver as Q2. Do not compare q3's gate executable timings to the
earlier timing-driver executable. Add explicit saved-corpus text-output memo
coverage (including nonadjacent repeated symbols) before generic acceptance.
No new full trace, identical-panel rerun, commit, push or release is justified.
