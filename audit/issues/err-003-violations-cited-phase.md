# err-003: `Violations.cited` is empty until `attach_cited` — a phase flag in the data

- **Severity:** medium
- **Tree:** err
- **Status:** OPEN
- **Source:** audit/storage-schema.md F11
- **Depends on:** none
- **Conflicts with:** err-002 (cited-facts parallel array)

## The bug

`error.rs:1038-1045,1084-1108` — "`Empty until the commit boundary's decode pass attaches it.`" `seal` / `one` inhabit an undecorated set; `cited_facts` returns `[]` for that phase and for an out-of-range index. `attach_cited` `assert_eq!`s parallel lengths. A rejection without decoded facts is representable, then accidentally empty at the bindings layer.

## Why it's wrong

Insight 6 — the decode pass learned the facts and stuffed them into a parallel array the type always allowed to be missing. Insight 4 — undecorated vs decorated as one product.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree.

- `Violations` (sealed citations, no cited facts) stays the collectors' / sweeper's type.
- `DecoratedViolations` (citations ∥ cited facts, lengths equal by construction) is what `Error::CommitRejected` carries. `attach_cited`'s assert becomes a constructor.
- Sweeper re-play that has no decode stays undecorated and is not `CommitRejected`.

## Acceptance criteria

- [ ] `Error::CommitRejected` cannot carry empty `cited` when citations are nonempty (constructor forbids it).
- [ ] Gone: `cited_facts` returning `[]` for "not yet decorated" on a `CommitRejected` value.
- [ ] Unchanged tests: commit-reject render tests still see decoded facts; sweeper findings still work without decoration.
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- Decode-at-reject-time (pending interns) stays inside the commit boundary. Citation sort/dedup/nonempty seal unchanged. Public `Violations` API may grow a type rather than silently changing `cited_facts` to panic.
