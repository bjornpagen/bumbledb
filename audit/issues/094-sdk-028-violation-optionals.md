# sdk-028: violation and statement-slot payloads are optionals on every kind

- **Severity:** medium
- **Tree:** sdk (ts db/native + napi marshal + cpp error)
- **Status:** OPEN
- **Source:** audit/sdk-rest.md #6
- **Depends on:** none (answers/write path; parallel-safe with query issues). raii `has_measure` *copy* stays sdk-008.
- **Conflicts with:** none at the dialect types; napi `ViolationWire` is the same file as sdk-008's query marshal — coordinate if both land.

## The bug

A rendered violation is a sum by form: FD (spelling + facts), containment (plus direction), capacity (plus measure), mirrors-slot (plus orientation). The hosts store a product of optionals.

TS `Violation` (`ts/src/db.ts:125-132`) and the wire type (`ts/src/native.ts:238-244`):

```typescript
interface Violation {
	readonly kind: StatementKindTag
	readonly direction?: "sourceUnsatisfied" | "targetRequired"
	readonly orientation?: "written" | "mirrored"
	readonly measure?: bigint
}
```

`StatementEntry` (`db.ts:440-451`) is the same product: `key?:` "exactly for functionality", `reversed?: boolean` "exactly for mirrors", `statement?:` undefined for implied keys. `orientationOf` (`db.ts:591-599`) is a flowchart on `boolean | undefined` producing a three-valued sum the type already had.

NAPI `ViolationWire` (`ts/crate/src/marshal.rs:1189-1194,1237-1242`) carries `direction: Option`, `measure: Option` and omits absent keys — a wire spelling that TS re-inflates into one interface.

C++ dialect `Violation` (`cpp/src/error.cc:84-90`) always carries `ViolationDirection` (dummy on FD/capacity) plus `std::optional<Measure>`; the fill (`:264`) branches on raii `has_measure`. The ABI `has_measure` + two u64 words is sdk-008 (essential C, not re-filed); the dialect type echoing it as a product is this issue.

FD-with-direction, capacity-with-orientation, and containment-with-measure are representable.

## Why it's wrong

Insight 4: three independent optionals admit eight states, a few valid; every consumer re-learns `kind` to know which payload is live. Insight 6: marshal / `render_rejection` already knew the form and threw the proof away. They already `switch` on `kind`.

## The fix

Per `audit/CONTRACT.md §C1` (trusted layers are sums):

- TS `Violation` / `StatementEntry` become sums, each arm carrying only its payload:

```typescript
type StatementEntry =
	| { kind: "functionality"; statement?: Statement; owner: string; projection: readonly string[] }
	| { kind: "containment"; statement: Statement }
	| { kind: "mirrors"; statement: Statement; orientation: "written" | "mirrored" }
	| { kind: "capacity"; statement: Statement }

type Violation<Rels> =
	| { kind: "functionality"; statement?: Statement; canonical: string; facts: … }
	| { kind: "containment"; statement?: Statement; canonical: string; direction: "sourceUnsatisfied" | "targetRequired"; facts: … }
	| { kind: "capacity"; statement?: Statement; canonical: string; measure: bigint; facts: … }
```

  (`mirrors` violations ride the containment arm plus `orientation`, or a dedicated arm — pick one, document it. `orientationOf` deletes.)

- C++ `Violation` matches on `kind`; `direction` lives in the containment arm; `measure` in the capacity arm. ABI `has_measure` stays (sdk-008); the dialect does not store it.
- NAPI may keep omitting absent keys on the wire object (JS has no sums); the TS *type* that consumes it is the sum, parsed at the host boundary (`db.ts` already walks `kind`).

## Acceptance criteria

- [ ] Gone: `rg -n 'direction\?:|orientation\?:|reversed\?:' ts/src/db.ts ts/src/native.ts` → no optional payloads on the unified Violation/StatementEntry product; `rg -n 'orientationOf' ts/src/db.ts` → no matches; C++ `struct Violation` has no always-present `ViolationDirection` beside an optional measure.
- [ ] Unchanged tests: `cd ts && pnpm test` and cpp `ctest` green; violation `canonical` strings and fingerprint lock unchanged. Tests that pinned optional-field presence update only if they asserted the bag shape.
- [ ] Green: `cd ts && pnpm test`; `cd ts/crate && PATH="$HOME/.cargo/bin:$PATH" cargo test`; `cd cpp && cmake --preset dev && cmake --build --preset dev && ctest --preset dev`.

## Constraints

- Semantics identical: direction relative to the violated slot, `orientation` distinguishing mirrors partners, measure u128-as-bigint (C3). ABI `bdb_violation.has_measure` layout frozen (sdk-008). Locked names untouched. No Program vocabulary.
