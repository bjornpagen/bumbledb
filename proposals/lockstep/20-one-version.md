# 20 — One version

> **Decision.** The release version is **one fact with one writer**: the
> root `[workspace.package] version`. Every crate inherits it
> (`version.workspace = true`). Every npm manifest — where cargo
> inheritance cannot reach — is checked against it by a gate that walks
> a **written roster of every versioned manifest in the repository**, so
> a manifest can be missing from the roster only by failing the gate
> that audits the roster against the tree. The pass ships as **0.19.0**,
> and the runbook is rewritten to say, first, that 0.19.0 stores are a
> new world.

## The current representation

The version is written by hand in fourteen places: eight engine-side
crate manifests, `crates/bumbledb-log/Cargo.toml`, `ts/crate/Cargo.toml`,
`ts/package.json`, two platform packages under `ts/npm/*`, and
`ts-log/package.json` — plus the *peer range* in `ts-log` that must
track it, and a prose runbook (`ts/PUBLISHING.md`) that narrates it.
The audit found the inevitable:

- `bumbledb-log` at `0.17.0` against everything else's `0.18.0` —
  **skew that shipped**, because the lockstep gate's implicit roster
  never included it (the two-workspace split, [10](10-one-workspace.md)).
- `PUBLISHING.md` describing `0.18.0` as the current release — correct
  prose about the past, silent about the release this tree actually is.

Fourteen hand-written copies of one fact is the `loading/error/data`
booleans of release engineering: the representation *admits* skew, so
skew occurs, so a guard (the lockstep gate) exists, and the guard itself
was wrong because its roster was a fifteenth unwritten copy.

## The target representation

### 1. Cargo side: inheritance, not discipline

The root manifest declares the one fact:

```toml
[workspace.package]
version = "0.19.0"
```

Every workspace crate — the eight engine crates and, after
[10](10-one-workspace.md), `bumbledb-log` — replaces its `version =`
line with `version.workspace = true`. The excluded foreign-build crates
(`ts/crate`, `bumbledb-c`) cannot inherit; they join the roster below.
On the cargo side, skew stops being a state the gate must catch and
becomes a state the manifest format cannot spell.

### 2. The roster: the gate's subject is data, not a glob

A checked-in roster (one file, one path per line) lists **every
versioned manifest in the repository**: the two excluded Cargo.tomls,
`ts/package.json`, `ts/npm/darwin-arm64/package.json`,
`ts/npm/linux-arm64/package.json`, `ts-log/package.json`. The lockstep
gate does two things, both trivial once the subject is data:

1. every manifest on the roster carries exactly the workspace version;
2. **the roster is complete** — a sweep of the tree for
   `version`-bearing manifests finds nothing off-roster. This second
   check is what was missing: it makes "a manifest the gate never met"
   a gate failure instead of a silent hole.

The `ts-log` peer range is derived, not written: the gate asserts
`peerDependencies["@bjornpagen/bumbledb"]` is exactly `^<workspace
version>`. A peer range that lags the release is a red gate, not a
release-notes surprise.

### 3. 0.19.0, and the runbook tells the truth first

The bump to `0.19.0` is one edit (the workspace field) plus the roster
manifests, one commit, lockfiles re-derived. `ts/PUBLISHING.md` gains
its 0.19.0 section, and its first paragraph is the one consumers need:

> **0.19.0 reads nothing 0.18.0 wrote.** The protocol documents are
> binary v:3 (`manifest`, `ckpt/{digest}`, `chain` — the `.json` keys
> are gone), the batch and document grammar is one binary language, and
> there is no migration path by design: re-checkpoint from a 0.19.0
> writer. This is the representation-first cutover shipping as one
> number.

The publish order, the platform packages, and the peer expectations
follow, updated from the 0.18.0 section's shape. The runbook remains
prose — it narrates a human ceremony — but every number in it is quoted
from the one fact, and the gate's roster is referenced instead of
re-listed.

## What gets deleted

| Deleted | Because |
| --- | --- |
| every hand-written `version = "0.x.y"` in workspace crate manifests | inherited from `workspace.package` |
| the lockstep gate's implicit walked-set | the roster file is the subject, and its completeness is checked |
| the hand-tracked `ts-log` peer range | derived: `^<workspace version>`, asserted by the gate |
| `PUBLISHING.md`'s 0.18.0-era numbers as live guidance | the 0.19.0 section, breaking-store banner first |

## The invariant

> **There is one version, it has one writer, and everything else that
> spells a version is either an inheritance the build system enforces or
> a roster entry the gate proves equal — and the roster itself is proven
> complete against the tree.** Version skew is not caught; on the cargo
> side it is unspellable, and on the npm side it is a red gate the same
> hour it is written.

Dissolves: audit B.1 (the 0.17.0 skew and the gate that missed it), B.2
(the peer range), B.3 (the stale runbook). Depends on
[10](10-one-workspace.md) for cargo inheritance to cover the log crate;
feeds [50](50-proof-as-gate.md), whose receipt quotes the one number.
