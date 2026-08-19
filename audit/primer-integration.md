# Primer integration audit

Lens: the unpublished Bumbledb 0.15 working tree against
`primer-spec/proposals/02-runtime-bumbledb-and-effect.md` and Phase 4 of
`primer-spec/proposals/00-unified-relational-oracle-library.md`.

This report records integration gaps only. It does not propose changes to the
admitted-instance representation. The heap builder and owned-instance kernel
exist and pass their focused Rust tests.

## Summary

| ID | Severity | Finding |
| --- | --- | --- |
| PRI-001 | Primer blocker | No safe runtime `QueryProgram` construction and preparation surface exists. |
| PRI-002 | Primer blocker | The public TypeScript heap-instance path has no end-to-end tests. |
| PRI-003 | Should fix before publish | `ParsedQuery` is a brand over raw numeric IR, not a constructed safe program representation. |
| PRI-004 | Should fix before publish | V8 external-memory accounting does not rise when a lazy image appears after admission. |
| PRI-005 | Should fix before publish | TypeScript still discards structured preparation and open refusal identity on some paths. |
| PRI-006 | Acceptance gate | The required full Primer corpus and scaling lane has not been demonstrated. |
| PRI-007 | Contract decision | Primer requires scoped prepared-program release, while the TypeScript SDK exposes prepared plans as GC-reclaimed plain values. |

---

### PRI-001 — safe runtime query programs are absent

**Severity:** Primer blocker.

**Location:**

- `ts/src/db.ts` — `Db.prepare`, `ReadInstance.prepare`, and
  `OwnedInstance.prepare` accept only the high-level typed `Query` value.
- `ts/src/query/parse-ir.ts` — `parseQueryIr` returns `ParsedQuery`.
- `ts/src/native.ts` — the prepareable raw bridge remains private.

**Required by Primer:** Phase 4 lowers one sealed, data-driven oracle catalog
into Bumbledb query programs. The compiler resolves relation names, field
names, output descriptors, and parameter descriptors at runtime. It must not
generate an unsafe cast or call the private native bridge.

**Current gap:** A host can build a statically typed `Query` through the fluent
builder. A host cannot safely turn a runtime oracle AST into a public
prepareable value. Public `parseQueryIr` does not close the gap because no
public `prepare` method accepts its result.

**Required surface:** Add one safe, schema-bound `QueryProgram` constructor.
It must:

1. Accept names rather than numeric relation and field identifiers.
2. Bind the complete program to one schema value.
3. Validate relation and field references.
4. Validate parameter and output descriptors.
5. Construct a public prepareable value.
6. Preserve the engine's canonical query rendering.
7. Return structured construction defects.

The constructor must not export the raw N-API bridge. It must not require an
unchecked assertion in a consumer.

**Acceptance:** A Primer law declaration lowers to a `QueryProgram`, prepares
against an `OwnedInstance`, executes, and returns its witnessed defect rows
without generated TypeScript or an unsafe cast.

---

### PRI-002 — no TypeScript heap-path conformance

**Severity:** Primer blocker before publication.

**Location:** `ts/test/`.

**Evidence:** No test calls `InstanceBuilder.create`, `InstanceBuilder.admit`,
or `Db.fromInstance`. The only `InstanceBuilder` occurrence in the TypeScript
tests is explanatory prose.

**Risk:** The focused Rust tests prove the native builder and owned instance.
They do not prove the public TypeScript composition across schema lowering,
row marshaling, async admission, typed admission mapping, prepared ownership,
answer decoding, disposal, or raw persistence.

**Required cases:**

1. Bulk-load several relations and admit an accepted candidate.
2. Admit a rejected candidate and inspect structured violations.
3. Prepare and execute a join against the accepted `OwnedInstance`.
4. Scan, contains, and keyed get through the owned instance.
5. Reject a prepared query from a different owned instance.
6. Dispose a builder and an owned instance and reject later use.
7. Persist with `Db.fromInstance` and reopen the resulting store.
8. Preserve duplicate semantic rows when an explicit occurrence attribute differs.

**Acceptance:** The public TypeScript lane executes the complete sequence
`InstanceBuilder -> load -> admit -> OwnedInstance -> prepare -> execute`, plus
rejection, disposal, foreign-plan, and persistence cases.

---

### PRI-003 — `ParsedQuery` does not carry a parsed representation

**Severity:** Should fix before publishing a runtime program surface.

**Location:**

- `ts/src/query/parse-ir.ts` — `return ir as ParsedQuery`.
- `ts/src/native.ts` — `ParsedQuery` is `QueryIr` plus a phantom brand.
- `audit/query.md` — existing Q-05 finding.

**Current gap:** The parser performs several boundary checks and then returns
the original wide numeric IR under a brand. The representation still admits
all combinations admitted by `QueryIr`. Later engine validation must recover
the narrower judgment.

**Why it matters to PRI-001:** A new public `QueryProgram` must not promote
this cast into the trusted consumer boundary. It needs a constructing parse or
an opaque engine-admitted program value.

**Acceptance:** The public runtime constructor produces a representation whose
only inhabitants passed the complete schema and query judgment. No exported
brand assertion is the proof.

---

### PRI-004 — lazy image memory remains invisible to V8

**Severity:** Should fix before publish.

**Location:**

- `ts/crate/src/lib.rs` — owned-instance memory is accounted at admission and
  released at close.
- `crates/bumbledb/src/api/db/owned.rs` — query execution may create lazy
  relation images after admission.
- `audit/bindings.md` — BND-02.

**Current gap:** Admission accounts the frozen catalog and any images already
present. Admission intentionally builds no relation images. Later query
execution can create the large images, but the external-memory count does not
rise with them.

**Risk:** A query-heavy Primer run can retain substantial Rust memory while V8
sees no corresponding pressure. This can delay garbage collection and obscure
the real memory curve.

**Acceptance:** External memory increases exactly when lazy image capacity is
retained and decreases when its last native owner releases it. The full Primer
lane reports catalog bytes and image bytes independently.

---

### PRI-005 — some TypeScript refusal paths remain stringly

**Severity:** Should fix before publish.

**Location:** `ts/src/db.ts` prepare and open refusal paths. **fixed this
pass** — wrap `ErrSchemaError` / `ErrFingerprintMismatch` / `ErrIrError`.

**Current gap:** The native bridge now preserves engine error families as
`{ kind, message }`. Some SDK paths receive a structured domain refusal and
immediately create a new message-only error such as a preparation refusal.

**Risk:** Primer's Effect layer must distinguish infrastructure failures from
domain admissions. It must not parse error messages to recover an error
family.

**Acceptance:** Every thrown SDK failure that consumers may classify retains
one structured identity. Domain outcomes remain discriminated values. No
consumer matches a rendered message.

---

### PRI-006 — the full Primer acceptance lane is unproved

**Severity:** Acceptance gate.

**Location:** `proposals/instance-lifetime.md`, allocation and performance
gates.

**Current gap:** Focused builder and owned-instance Rust tests pass. The
proposal additionally requires the full Primer normalization corpus through
bulk load, complete admission, keyed reads, representative joins, and raw
persistence. It also requires scaling through at least four corpus prefixes.
That evidence has not been produced by this integration review.

**Required measurements:**

1. Wall time.
2. CPU time.
3. Peak RSS.
4. Frozen catalog bytes.
5. Lazy image bytes.
6. Prepared, scratch, and answer capacity.
7. Entry count.
8. Allocation count.

**Acceptance:** The complete lane passes and the prefix series shows no
unexplained superlinear growth.

---

### PRI-007 — prepared-program lifetime contracts disagree

**Severity:** Cross-repository contract decision.

**Location:**

- `ts/src/db.ts` — `Prepared` is a plain value whose native handle is closed
  only by a `FinalizationRegistry` backstop.
- `primer-spec/proposals/02-runtime-bumbledb-and-effect.md` — the validator
  acquires and releases prepared programs through Effect scopes.

**Current gap:** A Primer scope can stop retaining a prepared value. It cannot
deterministically close the native prepared handle because the public value is
not disposable. The Bumbledb design intentionally makes builder, owned
instance, and witness disposable, but not prepared plans.

**Decision required:** Choose one contract before the Effect wrapper lands:

1. Give prepared programs deterministic public disposal and keep the Primer
   scope contract.
2. Keep prepared programs as GC-reclaimed values and revise the Primer scope
   contract explicitly.

Do not implement a private-handle escape hatch in Primer.

**Acceptance:** The two repositories state the same lifetime. A large law
roster has a demonstrated bound on retained native prepared-plan memory.

## Confirmed non-findings

- Heap construction itself is present.
- `InstanceBuilder.load` is collection-shaped.
- Complete admission returns a discriminated accepted or rejected value.
- `OwnedInstance` is immutable and queryable.
- Heap and LMDB reads use the same native query kernel.
- A rejected carrier or strict admission does not produce an owned instance.
- No Bumbledb FD, IND, containment, or query correctness bug surfaced in this review.

The missing runtime program constructor is an upstream feature gap. It is not
evidence of a native correctness bug.
