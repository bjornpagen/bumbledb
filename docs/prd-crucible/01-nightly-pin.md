# PRD 01 — The toolchain melts: pinned nightly, edition 2024

**Depends on:** baseline only. Everything downstream assumes this landed.
**Modules:** `rust-toolchain.toml`, every `Cargo.toml` (`edition`,
`[lints]`), `scripts/check.sh`, `scripts/check-asm.sh` (objdump paths per
toolchain), crate roots (`#![…]` attributes edition 2024 requires),
whatever the edition migration touches.
**Authority:** the owner's ruling of 2026-07-13: nightly, no questions
asked, no toolchain split, zero lingering stable consumers. The
measurement discipline (README): codegen changes invalidate pinned
margins — the register carries the re-earn session.
**Representation move:** the stable pin was about to force a permanent
dual — fuzz targets on nightly, everything else on 1.96 — and a dual
toolchain is a dual truth: two codegens, two sets of measured margins,
two CI stories. One pinned nightly deletes the split before it exists.

## Context (decided shape)

- `rust-toolchain.toml` pins **one dated nightly** — reproducibility is
  non-negotiable; a floating `nightly` channel would make every build a
  different compiler. Pick the newest nightly that (a) compiles the
  workspace, (b) has working `miri` and `llvm-tools` components on
  aarch64-apple-darwin, (c) cargo-fuzz accepts. Record the date and the
  three checks in the file's comment block, plus the standing rule: the
  pin moves deliberately (a PRD-sized action with the microbench re-earn
  attached), never implicitly.
- **Edition 2024** across all workspace crates and the fuzz crate to
  come. Known migration surface (verify each, fix directly — no
  `#[allow]` bridges): `unsafe_op_in_unsafe_fn` becomes deny-by-default
  (the image-decode and kernel `unsafe fn` bodies gain explicit
  `unsafe {}` blocks — an IMPROVEMENT: each unchecked operation gets its
  own scope and its `// SAFETY:` comment moves adjacent), RPIT lifetime
  capture rules (check `impl Iterator` returns in colt/iter, sweep,
  scan), `gen` becoming a reserved keyword (the bench crate has a `gen`
  module — rename to `r#gen` is FORBIDDEN as inelegance; the packet
  audit decided `corpus_gen`, which PRD 10 names as its later seam),
  match ergonomics changes, and
  `static_mut_refs` (expected: none — no static muts exist).
- `cargo fmt`/`clippy` move to the nightly versions — expect new lints;
  fix them, never blanket-allow (each new suppression follows the
  `#[expect]` + reason convention).
- `scripts/check.sh`: no logic change — it runs whatever toolchain the
  pin names. `check-asm.sh`: verify objdump/llvm-objdump resolution
  against the nightly's llvm-tools; the disassembly GATES themselves are
  content assertions and must pass unmodified — if a gate breaks because
  nightly codegen changed the hot loop, THAT IS A FINDING: stop, record
  per policy 5, and the human register's re-earn session adjudicates
  before the gate is edited.

## Technical direction

1. Land the pin + editions in one motion; `cargo build --workspace` is
   the worklist. Fix breakage per the known-surface list above, then
   whatever else surfaces — every fix direct, no bridging attributes.
2. Run the full workspace gate suite, then the separate asm gate. Diff
   the clippy lint set: new fires get real
   fixes or `#[expect(…, reason)]` with the reason argued.
3. Run `check-asm.sh` and the `#[ignore]`d microbenches once,
   informally, to size the codegen delta for the register's re-earn
   session (do not update any pinned numbers here — measurement is human
   work; just report what moved in the commit body).
4. The `gen`-keyword resolution: rename the module to `corpus_gen` and
   apply it everywhere (module path, imports, docs).

## Passing criteria

- `[shape]` `rust-toolchain.toml` names one dated nightly with the
  comment block (date, the three checks, the deliberate-move rule);
  `grep -rn 'edition' */Cargo.toml crates/*/Cargo.toml` → 2024
  everywhere; no `r#gen` anywhere.
- `[shape]` Zero new `#[allow]`; every new suppression is `#[expect]`
  with a reason; `unsafe fn` bodies contain explicit `unsafe {}` scopes
  with adjacent SAFETY comments.
- `[gate]` `scripts/check.sh` and `scripts/check-asm.sh` each exit 0 on
  the nightly pin; the asm gates remain unmodified (or a recorded
  conflict per direction 3, which BLOCKS this PRD until ruled).
- `[shape]` The commit body reports the informal microbench delta and
  names the re-earn session as pending human work.

## Recorded conflicts (policy 5, 2026-07-13)

1. **The asm gate misfires on v0 mangling, not on codegen.**
   `nightly-2026-07-12` defaults to v0 symbol mangling (1.96 used
   legacy). The outlined recursive `Colt::any_position<closure>` —
   called from `probe_pass`/`run_node` on BOTH toolchains, 12 identical
   `bl` sites in the 1.96 binary (verified by building HEAD pre-port at
   1.96.0 and disassembling) — was named `…12any_position17h…` under
   legacy mangling and is named
   `…any_positionNCNvB2_20any_position_matches0…` under v0. The gate's
   forbidden class `position_matches` now matches the *name* of the
   same machine code that always passed. The probe loop is unchanged;
   the per-element probe class carries no new calls. Per direction 3
   the gate is NOT edited here; `check-asm.sh` exits 1 on the new
   binary until the re-earn session rules how the forbidden patterns
   are re-expressed against v0/demangled names. This blocks PRD 01's
   asm-gate criterion pending that ruling.
2. **Two S-scale gate tests hang on a pre-existing liveness bug.**
   `driver::tests::the_full_sequence_runs_at_s` and
   `verify::tests::a_full_verify_at_s_succeeds` loop forever in the
   executor (pump/probe_pass ↔ AggregateSink dedup on one seeded
   random query). Reproduced identically on the unported stable binary
   (2.5 h hang), so the port is not the cause; a parallel lane owns the
   root-cause. `check.sh`'s stages were run individually: fmt --check,
   workspace clippy `-D warnings`, `cargo test --workspace` minus the
   two hung tests (0 failures), doc tests, the release alloc gate, the
   bench obs clippy + harness/trace_out/tripwires lanes — all exit 0;
   the cross check took the script's honest-skip branch (no cross C
   compiler on this host). The full unfiltered script re-runs when the
   liveness fix lands, at latest with PRD 09's reconciliation.

## Doc amendments (rule 5)

`00-product.md` (toolchain posture: one pinned nightly, the
deliberate-move rule, why the split was refused); repo `README.md` gate
section (toolchain line).
