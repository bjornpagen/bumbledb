## Closed-target key resolution rejects declared keys while citing them as available candidates

category: incoherence | severity: low | verdict: CONFIRMED | finder: engine:schema-api
outcome: fixed b7ddb7e0

### Summary

A containment targeting a closed relation is refused unless its target projection is exactly the synthetic id — an intended v0 rule ("the handle id is the one probe-able identity of a closed relation"). But the refusal reuses `SchemaError::NoMatchingTargetKey`, an error minted for "no key with this field set exists", and its `available` candidate roster is built with no closedness awareness. When the user has declared a key on the closed relation whose field set exactly equals the refused projection — which is legal, validated, and served by the point-read surface — the rendered error simultaneously claims the projection "matches no declared key" and lists that very key as available. The evidence contradicts the verdict, and the actual rule (closedness) is never named.

### Evidence

All verified against the working tree; the failure was reproduced through the public API.

- `crates/bumbledb/src/schema/validate.rs:1180-1186` — the closed arm of `resolve_target_key`:
  ```rust
  if let Some(rows) = target_relation.extension.as_deref() {
      if target.projection.len() != 1 || target.projection[0] != FieldId(0) {
          return Err(missing_target_key(id, target, descriptors, false));
      }
      ...
  }
  ```
  `missing_target_key(..., false)` (validate.rs:1308-1327) constructs `SchemaError::NoMatchingTargetKey` with `available: target_key_candidates(...)`.
- `crates/bumbledb/src/schema/validate.rs:1283-1306` — `target_key_candidates` pushes **every** `StatementDescriptor::Functionality` on the target relation into `available`: no closedness filter, no exclusion of the field set being refused.
- `crates/bumbledb/src/schema/validate.rs:579-631` — declared keys on closed relations are legal schema objects, judged once against the sealed extension ("A key on a closed relation is judged here, once").
- `crates/bumbledb/src/api/db/get.rs:109-129` — `closed_fact_by_determinant` serves dyn point reads through exactly such keys, so the declared key is live functionality, not a vestigial declaration.
- `crates/bumbledb/src/error/display.rs:36-69` — `target_key_rejection` renders "projection {…} matches no declared key; available keys: …", printing the contradiction verbatim.
- **Reproduction** (scratch crate against the public `SchemaDescriptor::validate`): closed `Kind { weight: U64 }` with two rows, declared `Kind(weight) -> Kind` (`FieldId(1)`, the sealed post-synthetic-id ordinal), and `Task(weight_ref) <= Kind(weight)`. Result:
  ```
  NoMatchingTargetKey { statement: StatementId(2), target: RelationId(0),
      projection: [FieldId(1)],
      available: [TargetKeyCandidate { key: KeyId(0), projection: [FieldId(0)] },
                  TargetKeyCandidate { key: KeyId(1), projection: [FieldId(1)] }] }
  ```
  Display: `statement 2: target relation 0 projection {1} matches no declared key; available keys: key 0 {0}; key 1 {1}` — key 1's field set is the refused projection.
- **Spec check** — `docs/architecture/30-dependencies.md`, § "IND into a closed target" and the rejection roster: the rule is "Y must be exactly the synthetic id (the handle is the one probe-able identity of a closed relation)", and the roster entry glosses the rejection as "no key matches". The gloss is only true when no declared key shares the field set; the code's unfiltered candidate roster falsifies it in the declared-key case. The existing test `rejects_a_closed_target_projection_that_is_not_the_id` (`crates/bumbledb/src/schema/tests/reject.rs:1126-1157`) only covers the coherent case — no declared key on the closed relation — so the contradiction is untested.

### Failure scenario

A user declares a closed relation with a payload key (`Kind(weight) -> Kind`, accepted and enforced at validate.rs:585-631, probe-able via `get`) and then writes `Task(weight_ref) <= Kind(weight)`. Validation fails with an error whose `available` list contains the exact projection the message says matches nothing. The user is sent hunting for a field-set typo that does not exist; the real rule — closed containment targets are addressed by handle id only — appears nowhere in the error.

### Suggested fix

The error variant is the representation; a truthful one erases the contradiction (parse-don't-validate applied to diagnostics). Either:

1. Mint a distinct typed rejection for the closed arm — e.g. `ClosedTargetNotHandle { statement, target, projection }` — whose display names the actual rule ("a closed target is addressed by its synthetic id; rewrite the target side as `Kind(id)`"), mirroring how `ClosedContainmentInterval` already names its own v0 refusal; or
2. If reusing the variant, make `target_key_candidates` closedness-aware for this arm so `available` carries only the auto-key `{id}` — the one candidate that is actually probe-able as a closed containment target.

Option 1 is the representation-first answer: the closed arm's refusal reason is closedness, a different fact than key absence, and the two deserve different encodings.
