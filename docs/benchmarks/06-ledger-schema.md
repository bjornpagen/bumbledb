# PRD 06 — The ledger schema

Authority: `50-validation.md` (owns the ledger), `10-data-model.md` (every construct
the schema must exercise), `00-product.md` (the benchmark is ledger-shaped by
decision).

## Purpose

The one benchmark schema, written down. Nine relations that exercise every engine
construct: all six value types, serials, single and compound uniques, single and
compound FKs (including the FK-inheritance pattern), enums, interned strings and
bytes, bools.

## Technical direction

`bumbledb_bench::schema` declares via the `schema!` macro (the macro is the blessed
surface and the bench is its biggest consumer):

```text
relation Currency   { id: u64 as CurrencyId, serial,  code: str, unique(code) }
relation Holder     { id: u64 as HolderId,   serial,  name: str,
                      region: enum Region { Na, Eu, Apac, Latam } }
relation Instrument { id: u64 as InstrumentId, serial, symbol: str,
                      currency: u64 as CurrencyId, fk(Currency.id),
                      kind: enum Kind { Cash, Equity, Bond, Fund } }
relation Account    { id: u64 as AccountId, serial,
                      holder: u64 as HolderId, fk(Holder.id),
                      currency: u64 as CurrencyId, fk(Currency.id),
                      status: enum Status { Open, Frozen, Closed },
                      opened_at: i64 }
relation Transfer   { id: u64 as TransferId, serial, at: i64, extref: bytes }
relation Posting    { id: u64 as PostingId, serial,
                      transfer: u64 as TransferId, fk(Transfer.id),
                      account: u64 as AccountId, fk(Account.id),
                      instrument: u64 as InstrumentId, fk(Instrument.id),
                      amount: i64, at: i64, memo: str, reconciled: bool }
relation Tag        { id: u64 as TagId, serial, label: str, unique(label) }
relation AccountTag { account: u64 as AccountId, fk(Account.id),
                      tag: u64 as TagId, fk(Tag.id),
                      unique(account, tag) }
relation TagNote    { account: u64 as AccountId,
                      tag: u64 as TagId,
                      fk(account, tag -> AccountTag.account_tag),
                      note: str }
```

- Rationale comments in the source: `Posting` is the fact table (the 10⁷ axis);
  `AccountTag` carries the compound unique; `TagNote` carries the compound-FK
  inheritance pattern; `Transfer.extref` is the Bytes exerciser; `unique(code)` /
  `unique(label)` are interned-string unique guards.
- `pub fn schema() -> &'static Schema` re-exported; relation-count and per-relation
  field-id constants where families need them (`pub mod ids` with documented
  consts — no magic numbers in family definitions).
- A golden fingerprint test pins the schema: changing it is a deliberate act that
  re-baselines every stored corpus and report (state this in the test's doc
  comment).

## Non-goals

Data (PRD 07). Any 10th relation "for realism" — nine is the envelope; additions
are owner decisions.

## Passing criteria

- Unit tests: `schema()` validates; the golden fingerprint bytes are pinned; a
  descriptor walk asserts all six `ValueType`s present, ≥1 compound unique, ≥1
  compound FK, ≥6 single FKs, ≥5 serials; `TagNote`'s FK targets
  `AccountTag.account_tag` (by resolved ids).
- `scripts/check.sh` green.
