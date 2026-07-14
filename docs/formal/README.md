# Formal semantics

**SUPERSEDED (the covenant campaign, PRD 01):** this artifact is the
statement inventory the living Lean specification in `lean/` was built
from; the living spec is `lean/`, and PRD 14 deletes this directory
(the SHA-pinned artifact remains reachable in git history forever).

`GPT55DependencyTheory.lean` is the formal companion to Bumbledb's
dependency and query semantics. It was produced by the gpt55 audit on
2026-07-13, pinned against repository commit `98f1103`, and checked with
`leanprover/lean4:v4.32.0` with no axioms and no `sorry` declarations. The
checked audit environment supplied the two imported precursor modules named by
the artifact.

The repository copy is byte-identical to the source artifact
`/Users/bjorn/Downloads/GPT55DependencyTheory.lean`:

```text
SHA-256 e1f09501079feb23ad93be9ab98aeba3b6b5f50a6a84cbbbf78af095c048a576
```

The model covers pure dependency and query semantics. It does not model
parsing, storage bytes, integer overflow, interning, or the completeness of a
closed extension; those remain Rust obligations, identified in the
[theorem-to-evidence table](../architecture/30-dependencies.md#formal-claims-and-runtime-evidence).

Re-running the Lean checker is registered human work. The repository does not
carry a Lean toolchain and Lean checking is not part of its automated gates.
