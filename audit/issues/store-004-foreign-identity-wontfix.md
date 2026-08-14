# store-004: `ForeignPreparedQuery` / `ForeignSnapshot` — essential runtime identity

- **Severity:** medium
- **Tree:** store
- **Status:** WONTFIX (essential; recorded by the audit itself)
- **Source:** audit/storage-schema.md F24
- **Depends on:** none

Cross-schema confusion is already unrepresentable (`Db<S>`). Cross-environment confusion is a process-distinct instance id — a runtime fact no static type can carry across two `&Db<S>` that share a lifetime (lifetime equality is not identity). `Witness<S>` already brands `write_from` with instance+generation (`api/db/write.rs:88-105,221-223`). `PreparedQuery` still key-probes `env_instance: u64` at execute.

`audit/CONTRACT.md` §C7 (docs F18): this is **essential runtime identity** with a named horizon — branding `PreparedQuery` with an environment/generation witness so a foreign snapshot fails at the call type *where the host language can express it*. That branding is not cheap (invariant-lifetime tokens still fail when both Dbs share `'a`; a unique brand type per `open` is an API). This issue does **not** mandate the engine change.

Not docs-018 (that is the docs sentence). This row is the engine occurrence of the same ruling, filed so a later campaign does not "fix" the check into a bool product or delete `ForeignPreparedQuery` / `ForeignSnapshot`. No edit under this id.
