# 19 — `attach_cited` still takes a parallel `Vec` and asserts the zip

- **Status:** **fixed this pass** — `Violations::from_pairs` takes the
  stored shape; both decoration loops (`catalog/decorate.rs`,
  `commit/write.rs`) build `(Violation, Box<[CitedFact]>)` pairs where
  they decode. The length `assert_eq!` is gone.
- **Severity:** later, small.
- **Supersedes:** the residue of PROP-011 (stored shape fixed) and PROP-013
  (compile-fail pin landed at `error.rs:1331`).

## Principle

The pairing is now the stored type
(`citations: Box<[(Violation, Box<[CitedFact]>)]>`) — but the decoration
constructor still accepts the two halves separately and re-asserts what the
parameter type could carry:

## Evidence

- `crates/bumbledb/src/error.rs:1373-1374` —
  `attach_cited(self, cited: Vec<Box<[CitedFact]>>)` opens with
  `assert_eq!` on the lengths.

## The fix

The decorator builds pairs where it decodes: `attach_cited` takes
`impl Iterator<Item = Box<[CitedFact]>>` zipped at the call site into
`(Violation, cited)` pairs — or better, the decoration loop constructs the
paired box directly and `attach_cited` becomes `from_pairs`. The `assert_eq!`
and its implicit panic path delete.

## Acceptance

- No length assert in `error.rs`'s decoration path; the pairing is carried
  by the parameter type.
