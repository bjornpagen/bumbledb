# Rust query notation corpus

Each JSON case records a query's source notation, normalized rendering,
production labels, and serialized query IR. `schema-fingerprint.txt` pins the
corpus schema.

The owning test is `crates/bumbledb-query/tests/notation_corpus.rs`. It compiles
the macro cases, prepares them against the engine schema, and compares the
rendered and serialized forms with the checked-in files.

```sh
cargo test -p bumbledb-query --test notation_corpus
```

For an intentional grammar change, inspect the changed expectations and use
the test's regeneration entry point:

```sh
cargo test -p bumbledb-query regenerate_the_notation_corpus -- --ignored
```

The former TypeScript `notation-corpus.test.ts` replayer no longer exists.
Do not describe this corpus as current two-language replay coverage. The
TypeScript SDK has separate query, boundary-codec, and installed-consumer
tests. Its public query builder constructs typed ASTs, not Rust notation or
SQL strings.

A changed golden needs an explained semantic or representation change;
regeneration alone does not establish correctness.
