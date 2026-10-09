# log-core lane board

Owns: `crates/bumbledb-log/**`, `docs/swarm/log-core.md`.

## Status

| Item | State |
|---|---|
| Delete the old log machine (writer, history, manifest, checkpointer, codec, gc, apply, recovery, backup, restore, replica, tenants, store, admin, transition, identities, inspect, erase, local_roots, certainty, json, schema_file, bindings, bins, conformance, all old tests; bench dev-dep; object_store/tokio/futures/once_cell_try) | done |
| Core types, one frame codec, Command, Entry | in progress |
| Sans-IO `Machine` (U11) + deterministic simulation | planned |
| LMDB cache over the engine host API | planned |
| Checkpoints (policy, images, cold open) | planned |
| Migrations (D6/D7) | planned |

## Notes for other lanes

- The old `bumbledb_log::*` API is gone. `crates/bumbledb-node` (bridge) no longer compiles against
  it until the bridge deletes its old log wire; the new API is announced below as it lands.
