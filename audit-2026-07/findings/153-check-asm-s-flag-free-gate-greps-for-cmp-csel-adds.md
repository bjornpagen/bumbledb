## check-asm's flag-free gate enumerates a spelling subset — subs/tst/cmn/ccmn/ands/fcmp and b.cond slip through

category: bug | severity: low | verdict: CONFIRMED | finder: r2:scripts-ci-packaging
outcome: fixed fca9e72a

### Summary

`scripts/check-asm.sh` exists to assert, against machine code, that the Allen hot-path kernels carry zero scalar flag-writing instructions — the flag-port-asymmetry law. But the gate's grep (line 49) enumerates five spellings — `cmp`, `csel`, `adds`, `ccmp`, `bl` — while aarch64 has several more NZCV-writing mnemonics that LLVM routinely emits: `subs` (printed instead of the `cmp` alias whenever the destination register is live, e.g. loop decrements), `ands`/`tst` (bit tests), `cmn`, `ccmn`, and `fcmp`/`fccmp`. Conditional branches (`b.<cond>`), which can only execute after something wrote NZCV, are also unchecked. The gate asserts a spelling subset, not the property the docs state as a class.

### Evidence (verified against the repo)

- `scripts/check-asm.sh:49` — the complete forbidden list:
  `if grep -E "[[:space:]](cmp|csel|adds|ccmp|bl)[[:space:]]" "$SYM" > "$BAD"`
- `scripts/check-asm.sh:31-37` — the stated law: "the Allen hot path carries zero scalar flag-writing instructions."
- Empirically tested the regex against objdump-format lines containing `subs`, `tst`, `cmn`, `ccmn`, `ands`, `fcmp`, and `b.ne`: **none matched**; only `cmp` did. (`fcmp` escapes because the pattern requires whitespace before `cmp`; `subs`/`cmn`/`ands` are distinct spellings; `b.ne` contains no `bl` token.) Additionally, `blr` (register-indirect call) slips the `bl` alternative, weakening the no-call-laundering clause at line 36-37.
- Doctrine check (the spec for this gate): `docs/architecture/40-execution.md:747` — "every flag-writing scalar op" follows the port-topology law; `docs/reference/apple-silicon-performance.md:206-207` (`m2max.core.flag-port-asymmetry`, `m2max.core.flag-strand-mlp`) define the taxed resource as flag µops as a class — flag ops execute on only 3 of 6 integer ALUs, and dependent flag µops parked behind misses halve sustainable MLP. The law is a class property; the gate checks a spelling list.
- Disassembled the actual `target/release/bumbledb-bench`: `allen_filter_batch_neon`'s loop control is `sub x8, x8, #1; cbnz` — flag-free (`cbz`/`cbnz` do not write NZCV), so the current binary passes the gate **legitimately**. The gap is latent, not active.

### Failure scenario

An LLVM upgrade (or a source tweak that makes the count live) compiles a kernel tail to `subs w9, w9, #1; b.ne` or a mask test to `tst w8, #0xf; b.eq`. Scalar flag traffic returns to the kernel — exactly the strand that halves gathered miss lanes per `m2max.core.flag-strand-mlp` — while the gate prints `ok … free of scalar flag writers` and CI stays green. The regression is then only catchable by the `#[ignore]`d microbench pins, defeating the point of a structural machine-code gate.

### Suggested fix

Assert the class, not the spellings. Two-layer pattern:

1. Add the remaining NZCV writers: `subs|ands|tst|cmn|ccmn|fcmp|fccmp` (and `adcs|sbcs|bics|negs` for completeness), plus `blr` alongside `bl`.
2. Add the structural witness: forbid `b\.[a-z]{2}` inside the kernel symbols — any condition-code branch proves a flag write occurred, whatever mnemonic produced it. The flag-free branches the kernels legitimately use (`cbz`/`cbnz`/`tbz`/`tbnz`) contain no dot and remain allowed, so the current binary still passes.

One line in the script comment (line 33-34) should also be updated from the four-spelling list to the class statement, matching `40-execution.md:747`.
