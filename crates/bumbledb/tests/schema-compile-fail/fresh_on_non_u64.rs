//! `fresh` is legal on u64 only (`docs/architecture/70-api.md` § field
//! syntax): the mint mark denotes a u64 generation, and the emitted
//! `Fresh`/`Key` impls are u64-shaped — so the macro judges the type at
//! expansion naming the field, never deferring to `Db::create` (the
//! deferral could not arrive: the generated impls would die as rustc
//! type errors in invisible code first).
//@ error: fresh field `id` must be u64

bumbledb::schema! {
    pub Minted;

    relation Record { id: i64 as RecordId, fresh, name: str }
}
