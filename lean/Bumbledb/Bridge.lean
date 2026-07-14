import Bumbledb.Values
import Bumbledb.Schema
import Bumbledb.Dependencies
import Bumbledb.Query.Syntax
import Bumbledb.Query.Denotation
import Bumbledb.Query.Aggregates
import Bumbledb.Exec.Sweep
import Bumbledb.Exec.Dedup
import Bumbledb.Exec.Rewrites
import Bumbledb.Txn

/-!
# Bridge — the obligation ledger (PRD 10)

The machine-listable ledger: each Lean premise paired with the Rust
mechanism that discharges it, replacing the prose theorem-to-evidence
table. It imports the whole tree because it indexes every theorem.

This file is a scaffold stub (PRD 01): the ledger lands in PRD 10.
-/
