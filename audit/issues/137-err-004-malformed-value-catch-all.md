# err-004: `CorruptionError::MalformedValue(&'static str)` is a catch-all kind

- **Severity:** medium
- **Tree:** err
- **Status:** FIXED(f074a598)
- **Source:** audit/storage-schema.md F17
- **Depends on:** none
- **Conflicts with:** store-003 (exhume armed-marker site)

## The bug

`error.rs:100-103` — `MalformedValue(&'static str)` "names which kind." The same file's `MetaMissing` vs `StoreKindInvalid` doctrine: "the two states point at opposite remedies, so one error value never encodes both." Then an armed ephemeral dirty marker (`storage/env/exhume.rs:76-79`) — a distinct remedy (wipe vs investigate) — is `MalformedValue("ephemeral dirty marker armed — …")`. Stringly typed corruption.

Census of non-width strings (kinds hiding in the catch-all, not "this counter failed to decode"):

- `exhume.rs:77` — `"ephemeral dirty marker armed — the store's last session never proved its sync"` (lifecycle; wipe vs investigate)
- `storage/dict.rs:99` — `"dict reverse id reuse"` (integrity: reverse map occupied; not a width)
- `api/db/exhume.rs:81` — `"descriptor round trip"` (codec fidelity after a successful decode; not a width)

Width/decode strings that **stay** `MalformedValue` (name the width): `"F key length"`, `"S row count"`, `"U determinant tail"`, `"U determinant key length"`, `"R key shape"`, `"R capacity value width"`, `"fresh-row key width"`, `"cited fact width"`, `"schema fingerprint"`, `"format version"`, `"tx id"`, `"dict forward id"`, `"dict next id"`, plus `read_meta.rs` / `descriptor_codec.rs` width names.

## Why it's wrong

Insight 4 — a string is every kind at once (Hoare, inverted). Distinct lifecycle states hide in the decode-failure arm. Future sites add more strings.

## The fix

`audit/CONTRACT.md` C1 does not freeze this tree. **Propose** a CONTRACT clause for corruption variants (one variant per distinct remedy; `MalformedValue` only for width/decode of a named counter or id). Do **not** number it C10 — capacity-laws C10 already means ray-Duration refusal (`docs/design/capacity-laws.md`, `Error::CapacityRayMeasure`). Implementable under C1–C8 without that pin: add the named variants locally.

Named variant per distinct remedy (`EphemeralDirtyArmed`, `DictReverseIdReuse`, `DescriptorRoundTrip`, plus any later census hit). `MalformedValue` stays only for "this counter/id failed to decode," with the static name of the *width*.

## Acceptance criteria

- [ ] Gone: `rg -n 'MalformedValue\("ephemeral dirty marker' crates/bumbledb/src`.
- [ ] Gone: `rg -n 'MalformedValue\("dict reverse id reuse' crates/bumbledb/src`; `rg -n 'MalformedValue\("descriptor round trip' crates/bumbledb/src`.
- [ ] A census: `rg -n 'MalformedValue\("' crates/bumbledb/src` — remaining strings name a decode width, not a lifecycle or integrity kind.
- [ ] Unchanged tests: exhume-armed-marker and decode-corruption tests still fail typed; Display may change to the new variant name (update Display tests only, not behavioral assertions).
- [ ] Green: `PATH="$HOME/.cargo/bin:$PATH" cargo test -p bumbledb`; `./scripts/check.sh`.

## Constraints

- `MetaMissing` / `StoreKindInvalid` / `DescriptorFingerprintDesync` stay distinct. Do not fold true decode failures into new variants unless the remedy differs. Display tests are the one allowed assertion edit class.
- Do not pretend proposed corruption-variant numbering is in CONTRACT.md.
