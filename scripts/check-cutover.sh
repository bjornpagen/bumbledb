#!/usr/bin/env bash
set -euo pipefail

! rg "candidate_values_for_variable|collect_atom_candidates|BTreeSet<EncodedValue>" crates/bumbledb-lmdb/src
! rg "scan_encoded_index_prefix" crates/bumbledb-lmdb/src/query.rs
! rg "execute_atoms|execute_atom|ChosenAccess|PlannedAtom|plan\.atoms|wcoj|WCOJ" crates/bumbledb-lmdb/src
