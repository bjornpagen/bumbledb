## RelationSpec splits closedness across two parallel Options, admitting two states the macro grammar makes unrepresentable

category: incoherence | severity: low | verdict: CONFIRMED | finder: theory

### Summary

`RelationSpec` in `crates/bumbledb-theory/src/schema/spec.rs` carries `newtype: Option<Box<str>>` and `extension: Option<Vec<RowSpec>>` as two independent fields. The design intends them coupled — the newtype is "the macro's mandatory `as NewType`" for closed relations, and its own doc admits it is "Meaningless on an ordinary relation." Because the pair is not fused, two states the grammar forbids are representable, and the lowering pass tolerates both with silent `continue`s rather than typed `SpecIssue`s. This contradicts two of the crate's own doctrines: the kind-is-the-option law ("No relation-kind enum exists: the option *is* the kind" — `RelationDescriptor` doc in `schema.rs`), and the canonical-utterance ban table, under which every other say-nothing spelling (`WindowVacuous`, `DegenerateLiteralSet`, `RowArityExcess`, degenerate literal sets) is a typed issue, never a skip.

### Evidence (all verified in-repo)

- `crates/bumbledb-theory/src/schema/spec.rs:54-67` — the two parallel Options; the `newtype` doc (:56-61) ends "Meaningless on an ordinary relation."
- `crates/bumbledb-theory/src/schema/spec.rs:781-787` — the handle-namespace pass in `descriptor()`:
  ```rust
  if relation.extension.is_none() { continue; }
  let Some(newtype) = relation.newtype.as_deref() else { continue; };
  ```
  Both illegal states fall through with no issue. The complete `SpecIssue` push set in this file (`:502, :522, :598, :628, :641, :681, :713, :718, :722, :730, :735, :789, :828`) contains no variant for either state.
- `relation.newtype` is read at exactly two sites (`spec.rs:540`, `spec.rs:785`), both guarded by `extension.is_some()` — on an ordinary relation the field is fully dead data.
- Macro parity broken: `crates/bumbledb-macros/src/lib.rs:519-528` — `parse_closed_relation` asserts `as NewType` ("schema!: closed relation `{name}` needs `as NewType` — the handle needs a host type"), and ordinary-relation grammar has no relation-level `as` slot at all. The macro lowers through this very `SchemaSpec` (`lower_relations`, lib.rs:1368-1425, always sets `newtype` from the mandatory parse on closed relations), so the two silent branches are reachable only from foreign hosts — exactly the callers the spec exists to serve.
- Doctrine: `crates/bumbledb-theory/src/schema.rs` (`RelationDescriptor` doc): "No relation-kind enum exists: the option *is* the kind (`docs/architecture/10-data-model.md`)."
- Contract doc checked: `docs/architecture/70-api.md` § "The SchemaSpec bindings contract (normative)" says the spec "mirrors the grammar one-for-one" and that the macro and spec "produce indistinguishable descriptors"; nothing there sanctions relaxing the closed-relation newtype mandate on the spec path (the RelationSpec bullet doesn't even document the relation-level newtype field).
- Test coverage: `crates/bumbledb/tests/schema_spec.rs` pairs `newtype: Some(..)` with `extension: Some(..)` in every closed relation; neither illegal state is exercised anywhere.

### Failure scenario

- A foreign host (the Node bindings, ETL tooling) sets `newtype: Some("StatusId")` on an ordinary relation, expecting handle literals to resolve through it. Lowering ignores the field entirely; a later `LiteralSpec::Handle("Frozen")` on a field labeled `StatusId` fails as `NotAHandleField`. (Minor softening of the original claim: that error's message does say handles are "legal only on a field whose newtype is a closed relation's handle newtype" — but the declaration-site mistake, the useless relation-level newtype, is never diagnosed, so the host is pointed at the selection, not at the declaration that caused it.)
- Symmetrically, a closed relation minted with `newtype: None` lowers cleanly and validates (the descriptor carries no newtypes), but its handle namespace entry is never created (spec.rs:785-787 skips it), so its rows' handles are permanently unreachable from any `LiteralSpec::Handle` — with no error at declaration time at all.

### Suggested fix

Fuse the pair so the representation erases both states (the project's representation-first doctrine, applied to its own contract type):

```rust
pub struct ClosedSpec { pub newtype: Box<str>, pub rows: Vec<RowSpec> }
pub struct RelationSpec { pub name: Box<str>, pub fields: Vec<FieldSpec>, pub closed: Option<ClosedSpec> }
```

Ordinary-with-newtype becomes unrepresentable and every closed relation carries its handle newtype by construction; the two silent `continue`s at spec.rs:781-787 collapse into plain iteration over `closed`. If the spec instead deliberately relaxes the macro's mandate (closed-but-nameless as a legal foreign-host shape), that relaxation must be stated in the spec doc and in 70-api.md's contract section, and the ordinary-with-newtype case still needs either fusion or a typed `SpecIssue` — the current shape documents the coupling in prose while denying it in the type.
