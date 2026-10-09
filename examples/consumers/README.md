# Installed-package consumers

- `core-ts/consumer.ts`: the TypeScript SDK as an installed package would use it: schemas,
  changes, bounded reads and reusable queries over `@bjornpagen/bumbledb/engine`.
- `rust/`: a standalone Cargo consumer of the `bumbledb` crate. It sits outside the workspace and
  depends on the crate by path, because the workspace crates are not published.
