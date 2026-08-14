# err-004: `CorruptionError::MalformedValue(&'static str)` is a catch-all kind

- **Severity:** medium
- **Tree:** err
- **Status:** OPEN
- **Source:** audit/storage-schema.md F17
- **Depends on:** none
- **Conflicts with:** store-003 (exhume armed-marker site)

## The bug

`error.rs:100-103` — `MalformedValue(&'static str)` "names which kind." The same file's `MetaMissing` vs `StoreKindInvalid` doctrine: "the two states point at opposite remedies, so one error value never encodes both." Then an armed ephemeral dirty marker (`storage/env/exhume.rs:76-79`) — a distinct remedy (wipe vs investigate) — is `MalformedValue("ephemeral dirty marker armed — …")`. Stringly typed corruption.

## Why it's wrong

Insight 4 — a string is every kind at once (Hoare, inverted). Distinct lifecycle states hide in the decode-failure arm. Future sites add more strings.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree. **CONTRACT gap — propose C10** (corruption variants: one variant per distinct remedy; `MalformedValue` only for width/decode of a named counter or id).

Named variant per distinct remedy (`EphemeralDirtyArmed`, plus any other string that is really a kind). `MalformedValue` stays only for "this counter/id failed to decode," with the static name of the *width*.

## Acceptance criteria

- [ ] Gone: `rg -n 'MalformedValue\("ephemeral dirty marker' crates/bumbledb/src`.
- [ ] A census: `rg -n 'MalformedValue\("' crates/bumbledb/src` — remaining strings name a decode width, not a lifecycle.
- [ ] Unchanged tests: exhume-armed-marker and decode-corruption tests still fail typed; Display may change to the new variant name (update Display tests only, not behavioral assertions).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- `MetaMissing` / `StoreKindInvalid` / `DescriptorFingerprintDesync` stay distinct. Do not fold true decode failures into new variants unless the remedy differs. Display tests are the one allowed assertion edit class.
