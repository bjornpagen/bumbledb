import Bumbledb.Dependencies

/-!
# Txn — the lifecycle (Level 2, PRD 09)

The transaction state machine: op-order invariance, final-state
judgment, generation witnesses, snapshot isolation, the ETL identity.
Committed-state transitions only — durability and crash belong to the
crashpoint estate, never here.

This file is a scaffold stub (PRD 01): the state machine and its
invariance theorems land in PRD 09.
-/
