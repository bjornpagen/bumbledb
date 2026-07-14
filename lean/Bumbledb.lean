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
import Bumbledb.Bridge
import Bumbledb.Countermodels

/-!
# Bumbledb — the formal specification

The root import file: building this module builds the entire tree.
This tree is the ONLY normative home of bumbledb's semantics; the
architecture docs cite it and never restate it. See `lean/README.md`
for the refinement chain and the laws.
-/
