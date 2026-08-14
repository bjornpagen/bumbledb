# sdk-20: variable finds store a dummy `op = fold_form::sum`

Severity: low
Tree: sdk (cpp)
Status: OPEN
Source: audit/sdks.md #20
Blocked-by: sdk-04
Blocks: none

## Bug

`cpp/src/query/rule.cc:229-232`; `query_view.cc:184-193`: variable
finds fill `op = fold_form::sum`; `head_term_of` writes
`BDB_HEAD_OP_SUM` on Var heads "because the field exists" — a
consumer that drops the tag reads Sum.

## Fix

Cites CONTRACT C6: sdk-04's four-case find sum — Var carries no op;
the filler and the unconditional ABI write die (the ABI field may
still exist per C layout; it is written only on aggregate kinds, and
readers key on the tag).

## Acceptance criteria

- [ ] Grep `fold_form::sum` as a filler on variable finds returns
      empty.
- [ ] Bridge tests green; wire/ABI behavior for aggregate finds
      unchanged.

## Constraints

Rider on sdk-04. No Program vocabulary.
