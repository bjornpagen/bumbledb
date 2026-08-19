//! The embedding surface (the 60-api doc, `docs/architecture/70-api.md`): [`Db`],
//! the read/write transaction closures, the typed write path, and export.
//!
//! Threading doctrine (`00-product.md`): the engine owns zero threads. The
//! handle is `Send + Sync`; readers run concurrently on LMDB snapshots;
//! writes queue on one mutex. A write transaction is a [`WriteDelta`]
//! — in-memory set arithmetic, nothing touches LMDB until commit, and an
//! abort (error or panic) never wrote a fact: the one thing every abort
//! persists is the escaped fresh high-water, burned exactly once by the
//! `EscapedIdBurn` drop guard (`db/write.rs`) — the never-reissue law
//! binds a panicking closure like any other termination.
//!
//! # The `Fact<'a>` lifetime (a decision, recorded here)
//!
//! Macro-generated fact structs borrow their variable-width fields
//! (`str` → `&'a str`, `bytes` → `&'a [u8]`), so the trait must express
//! "the struct is generic over a lifetime". Of the two shapes that can,
//! the **lifetime-parameterized trait** (`impl<'a> Fact<'a> for
//! Account<'a>`) wins over a GAT (`type Borrowed<'a>`): decode returns
//! plain `Self` at the resolver's lifetime with no projection type
//! anywhere, insert takes the struct at whatever lifetime the host holds
//! (an `Account<'b>` implements `Fact<'b>` — the encode paths read the
//! fields as borrows and force nothing toward `'static`), and
//! all-fixed-width structs implement `Fact<'a>` for every `'a` without
//! growing a lifetime themselves. A GAT would buy a lifetime-free trait
//! at the price of a second name for every struct (`F::Borrowed<'a>` next
//! to `F`) — an owned/borrowed twin in all but syntax, which the design
//! refuses. There are no owned twins and no modes: the borrowed struct is
//! the struct, and ownership is an explicit host act (`to_owned()` on the
//! field you keep).

use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::encoding::{InternId, ValueRef};
use crate::error::Result;
use crate::image::cache::ImageCache;
use crate::schema::Schema;
use crate::storage::catalog::CatalogRead;
use crate::storage::delta::WriteDelta;
use crate::storage::env::{Environment, ReadTxn};
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

mod apply;
mod builder;
mod delete;
mod delete_dyn;
mod encode_dyn;
mod exhume;
mod get;
mod insert;
mod insert_dyn;
mod instance;
mod maintain;
mod mutation;
mod mutation_core;
mod open;
mod owned;
mod prepare;
mod read;
mod read_instance;
mod reserve;
mod write;

pub use builder::InstanceBuilder;
pub use exhume::{Exhumed, exhume};
pub use instance::Instance;
pub use mutation::{FreshRange, FreshRangeIter, MutationReport};
pub use owned::OwnedInstance;
pub use write::Witness;

/// House `PutOutcome` applied to the codec: a dictionary miss proves the
/// fact (or determinant) absent rather than a Boolean whose polarity
/// callers must remember.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Probe {
    Encoded,
    ProvablyAbsent,
}

mod codec_seal {
    pub trait Sealed {}
}

/// Codec dependencies for encode-probe and decode. Names are
/// dependencies, not backends — a codec generic over this cannot branch
/// on the concrete catalog.
#[doc(hidden)]
pub trait CodecRead<S>: codec_seal::Sealed {
    fn schema(&self) -> &Schema;
    fn lookup_str(&self, value: &str) -> Result<Option<InternId>>;
    fn resolve_str(&self, id: InternId) -> Result<&str>;

    fn decode_bool_field(&self, relation: RelationId, fact: &[u8], idx: usize) -> Result<bool> {
        Ok(crate::encoding::decode_bool_at(
            view(self.schema(), relation, fact),
            idx,
        )?)
    }

    fn decode_u64_field(&self, relation: RelationId, fact: &[u8], idx: usize) -> Result<u64> {
        Ok(crate::encoding::decode_u64(
            crate::encoding::field_word_bytes(view(self.schema(), relation, fact), idx),
        ))
    }

    fn decode_i64_field(&self, relation: RelationId, fact: &[u8], idx: usize) -> Result<i64> {
        Ok(crate::encoding::decode_i64(
            crate::encoding::field_word_bytes(view(self.schema(), relation, fact), idx),
        ))
    }

    fn decode_str_field(&self, relation: RelationId, fact: &[u8], idx: usize) -> Result<&str> {
        let id = InternId::from_raw(crate::encoding::decode_u64(
            crate::encoding::field_word_bytes(view(self.schema(), relation, fact), idx),
        ));
        self.resolve_str(id)
    }

    fn decode_fixed_bytes_field<'f>(
        &self,
        relation: RelationId,
        fact: &'f [u8],
        idx: usize,
    ) -> Result<&'f [u8]> {
        Ok(crate::encoding::decode_fixed_bytes(
            view(self.schema(), relation, fact),
            idx,
        )?)
    }

    fn decode_interval_u64_field(
        &self,
        relation: RelationId,
        fact: &[u8],
        idx: usize,
    ) -> Result<crate::Interval<u64>> {
        Ok(crate::encoding::decode_interval_u64(
            view(self.schema(), relation, fact),
            idx,
        )?)
    }

    fn decode_interval_i64_field(
        &self,
        relation: RelationId,
        fact: &[u8],
        idx: usize,
    ) -> Result<crate::Interval<i64>> {
        Ok(crate::encoding::decode_interval_i64(
            view(self.schema(), relation, fact),
            idx,
        )?)
    }
}

/// Codec write dependency: mint intern ids. Extends [`CodecRead`] so
/// insert encoding cannot branch on a backend the type does not name.
#[doc(hidden)]
pub trait CodecWrite<S>: CodecRead<S> {
    fn intern_str(&mut self, value: &str) -> Result<InternId>;
}

fn view<'s, 'f>(
    schema: &'s Schema,
    relation: RelationId,
    fact: &'f [u8],
) -> crate::encoding::FactView<'f, 's> {
    schema.relation(relation).layout().encoded(fact)
}

/// One typed fact struct, as generated by the `schema!` macro. The write
/// side encodes against a [`CodecWrite`] (interning novel strings); the
/// probe side encodes against a [`CodecRead`] and reports a never-interned
/// value as [`Probe::ProvablyAbsent`].
///
/// `'a` is the decode lifetime: variable-width fields (`&'a str` /
/// `&'a [u8]`) borrow from the resolver — the snapshot's committed
/// dictionary (mmap pages, txn-stable by LMDB `CoW`) or the write
/// transaction's pending interns (delta arena). A struct with no
/// variable-width field implements `Fact<'a>` for every `'a`. The trait
/// shape is a recorded decision — see the module doc.
pub trait Fact<'a>: Sized {
    /// The schema this struct belongs to — the `schema!` invocation's
    /// named unit struct. Write and read operations bound
    /// `F: Fact<'_, Schema = S>` against `Db<S>`, so a cross-schema fact
    /// is a compile error.
    type Schema;

    /// The relation this struct declares, by declaration order.
    const RELATION: RelationId;

    /// Encodes the canonical fact bytes against a write context, interning
    /// novel strings through the codec. Appends to `out`.
    ///
    /// # Errors
    ///
    /// Storage errors from the dictionary reads.
    fn encode_insert<C>(&self, context: &mut C, out: &mut Vec<u8>) -> Result<()>
    where
        C: CodecWrite<Self::Schema>;

    /// Encodes against a read context. [`Probe::ProvablyAbsent`] means a
    /// string value was never interned — the fact cannot exist (`out` is
    /// left unusable in that case).
    ///
    /// # Errors
    ///
    /// Storage errors from the dictionary reads.
    fn encode_probe<C>(&self, context: &C, out: &mut Vec<u8>) -> Result<Probe>
    where
        C: CodecRead<Self::Schema>;

    /// Decodes canonical fact bytes back into the typed struct.
    /// Variable-width fields borrow from the codec's dictionary at `'a`;
    /// UTF-8 is validated at resolve (parse, don't validate) — without a
    /// copy. Pending-first vs committed-only resolution is carried by
    /// `C`, not by a second method.
    ///
    /// # Errors
    ///
    /// `Corruption` on undecodable bytes or dangling intern ids.
    fn decode<C>(context: &'a C, fact: &[u8]) -> Result<Self>
    where
        C: CodecRead<Self::Schema>;
}

/// A typed key value: one key FD's determinant, carrying the relation's
/// fact type — `snap.get(key)` / `tx.get(key)` return `Option<Self::Fact>`.
/// Generated by `schema!`: every fresh newtype implements it (reading
/// through its auto-materialized `R(field) -> R`), and every declared
/// `R(x, ..) -> R` statement yields a generated key struct (KG-2).
/// `'a` is the RESULT borrow (the snapshot's dictionary / the write
/// transaction's pending interns), independent of any borrow the key
/// value itself carries.
///
/// The key value's TYPE answers what used to be runtime questions:
/// which relation an id reads, and which statement disambiguates several
/// key FDs over one newtype — two keys over the same newtype are two
/// distinct Rust types. A cross-schema key is a compile error, not a
/// runtime check:
///
/// ```compile_fail
/// bumbledb::schema! {
///     pub SchemaA;
///     relation Left { id: u64 as LeftId, fresh }
/// }
/// bumbledb::schema! {
///     pub SchemaB;
///     relation Right { id: u64 as RightId, fresh }
/// }
/// // A key of `SchemaA` cannot read through a `Db<SchemaB>` read lease:
/// // `LeftId: Key<'_, Schema = SchemaB>` does not hold.
/// fn cross(db: &bumbledb::Db<SchemaB>, id: LeftId) -> bumbledb::Result<Option<Left>> {
///     db.read(|snap| snap.get(id))
/// }
/// ```
///
/// And the dead turbofish spelling no longer typechecks — the key VALUE
/// carries the fact type, so the call is `tx.get(id)`:
///
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     relation Account { id: u64 as AccountId, fresh, balance: i64 }
/// }
/// fn probe(db: &bumbledb::Db<Ledger>, id: AccountId) -> bumbledb::Result<()> {
///     db.write(|tx| {
///         // (`::` spaced apart: the repo bans the dead spelling textually)
///         let _ = tx.get :: <Account>(id)?;
///         Ok(())
///     })
/// }
/// ```
pub trait Key<'a>: Sized {
    /// The schema the key's relation belongs to (same closure as
    /// [`Fact::Schema`]).
    type Schema;

    /// The fact this key determines (at most one row).
    type Fact: Fact<'a, Schema = Self::Schema>;

    /// The materialized key statement this value reads through — computed
    /// at `schema!` expansion from the one shared lowering's materialized
    /// order, never discovered at runtime.
    const STATEMENT: StatementId;

    /// Appends the determinant bytes (canonical field encodings in
    /// statement projection order), resolving interned values through the
    /// codec. [`Probe::ProvablyAbsent`] = a string value was never interned:
    /// no fact can carry it.
    ///
    /// # Errors
    ///
    /// Storage errors from the dictionary reads.
    fn encode_determinant<C>(&self, context: &C, out: &mut Vec<u8>) -> Result<Probe>
    where
        C: CodecRead<Self::Schema>;
}

/// A fresh-minted newtype, as generated by the `schema!` macro for a
/// `fresh` field declared `as NewType`: [`WriteTx::reserve`] mints the next
/// values with the field already known.
pub trait Fresh: Sized + Copy {
    /// The schema the newtype's relation belongs to — [`WriteTx::reserve`]
    /// bounds `T: Fresh<Schema = S>`, so minting through a foreign
    /// schema's newtype is a compile error (the same closure as
    /// [`Fact::Schema`]).
    type Schema;

    /// The relation owning the fresh field.
    const RELATION: RelationId;
    /// The fresh field itself.
    const FIELD: FieldId;
    /// Wraps a minted raw value.
    fn from_fresh(raw: u64) -> Self;
    /// The raw value back out — the encode side of [`WriteTx::get`]'s
    /// key lookup (fresh newtypes are `Copy`).
    fn fresh(self) -> u64;
}

/// The database handle: the LMDB environment, the image cache, and the
/// writer mutex. Shareable across threads (`Send + Sync`); dropping the
/// handle closes the environment.
///
/// `S` is the schema definition ([`crate::Theory`]) the database was
/// created or opened with — a phantom typestate threaded through
/// [`WriteTx`], [`ReadInstance`], and [`crate::PreparedQuery`], so a fact or
/// prepared query of one schema cannot reach a database of another
/// (compile error, not a runtime width check). The validated
/// [`Schema`] itself is owned by the handle: `create`/`open` validate the
/// definition's descriptor and surface an invalid declaration as the
/// typed [`crate::error::SchemaError`].
///
/// Transaction closures return [`crate::Result`]; host code with its own
/// error type wraps the transaction instead of threading the type
/// through: run the closure for the engine work, then convert — a
/// generic error parameter here would force turbofish annotations on
/// every plain `Ok(())` closure.
pub struct Db<S> {
    /// The reader cache: one parked LMDB read
    /// transaction, reused while no commit has intervened. Sound because
    /// this handle is the environment's ONLY writer (exclusive lock at
    /// open): if [`Db::generation`] is unchanged since the parked
    /// reader began, the parked lease is bit-identical to a fresh
    /// one — and the per-read `mdb_txn_begin` (the point path's last
    /// fixed cost) is skipped entirely. Readers
    /// `try_lock`: contended readers fall back to a fresh transaction,
    /// never block.
    ///
    /// Declared BEFORE `env` — fields drop in declaration order, and
    /// the parked transaction owns its own env clone: it must close
    /// before `env` releases the advisory lock, or the drop opens a
    /// window where another handle acquires the lock while heed still
    /// holds the path open and surfaces `EnvAlreadyOpened` as an
    /// untyped `Lmdb` error (pinned by
    /// `dropping_the_handle_never_leaks_an_env_already_opened_window`).
    read_cache: Mutex<Option<ParkedReader>>,
    env: Environment,
    cache: ImageCache,
    writer: Mutex<()>,
    /// The thread currently inside [`Db::write`]: a nested `write` on the
    /// same thread would self-deadlock on the writer mutex forever, so it
    /// panics loudly instead. [`ThreadKey`] stored as `AtomicU64`; `0` is
    /// the cell's empty encoding, never a legal key.
    writer_thread: std::sync::atomic::AtomicU64,
    /// Last state-changing [`crate::GenerationId`] this handle has
    /// observed. The parked reader keys on it — the same clock the
    /// store advances, not a second process counter.
    generation: std::sync::atomic::AtomicU64,
    schema: Arc<Schema>,
    /// The typestate marker (`fn() -> S` keeps `Send + Sync` independent
    /// of `S` — the definition value itself is consumed at open).
    marker: PhantomData<fn() -> S>,
}

impl<S> Db<S> {
    /// The LMDB environment (reader: `crate::verify_store` — the sweeper
    /// opens its own read transaction, and its fixture tests inject raw desyncs
    /// through the environment's write transactions).
    pub(crate) fn env(&self) -> &Environment {
        &self.env
    }

    /// The sealed schema this handle was admitted under.
    #[must_use]
    pub fn schema(&self) -> &Schema {
        self.schema.as_ref()
    }

    /// The non-fatal declaration diagnostics validation sealed into this
    /// handle's schema witness ([`crate::schema::SchemaWarning`],
    /// `docs/architecture/70-api.md` § schema warnings): construction
    /// validates the descriptor and owns the witness, so the handle is
    /// where the sealed diagnostics surface — a borrow, zero recompute.
    /// Warnings are never errors and never alter the fingerprint.
    #[must_use]
    pub fn schema_warnings(&self) -> &[crate::schema::SchemaWarning] {
        self.schema.warnings()
    }

    /// Renders a query in the rule notation ([`crate::ir::render`]) —
    /// the roster-error diagnostic surface: when [`Db::prepare`] rejects
    /// a query there is no prepared handle to ask, so the host renders
    /// the offending query here and prints it beside the typed error
    /// (the statement renderer's precedent: errors cite the algebra).
    /// Total on malformed queries — unknown ids render as
    /// `relation#N`/`field#N` placeholders. Allocates; diagnostics only.
    #[must_use]
    pub fn render_query(&self, query: &crate::ir::Query) -> String {
        crate::ir::render::render(&self.schema, query)
    }
}

/// One parked read lease and the generation it saw.
struct ParkedReader {
    txn: heed::RoTxn<'static, heed::WithoutTls>,
    generation: crate::GenerationId,
}

/// One `ReadInstance` point read's reusable buffers: the composed `U` key /
/// encoded fact bytes, and the dyn membership probe's column refs.
#[derive(Default)]
pub(crate) struct ReadScratch {
    pub(crate) bytes: Vec<u8>,
    pub(crate) refs: Vec<ValueRef>,
}

/// The `Db`-owned point-read scratch pool (`docs/architecture/70-api.md`
/// § the write path — the allocation contract is symmetric across
/// transaction kinds, ruled 2026-07-23, R15): `ReadInstance` point reads take
/// a scratch set and restore it — the `&self` twin of
/// [`WriteTx::with_scratch`]'s take/restore. One entry per concurrent
/// point reader; contention grows the pool once, then the steady state
/// allocates nothing. `try_lock` on both sides: readers never block on
/// each other's scratch.
pub(crate) struct ScratchPool(Mutex<Vec<ReadScratch>>);

impl ScratchPool {
    pub(crate) fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    fn take(&self) -> ReadScratch {
        self.0
            .try_lock()
            .ok()
            .and_then(|mut pool| pool.pop())
            .unwrap_or_default()
    }

    fn restore(&self, mut scratch: ReadScratch) {
        scratch.bytes.clear();
        scratch.refs.clear();
        if let Ok(mut pool) = self.0.try_lock() {
            pool.push(scratch);
        }
    }
}

/// Process-wide thread identity for the nested-write detector. Never zero;
/// absence of a writer is `None`, not a sentinel in the atomic cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThreadKey(std::num::NonZeroU64);

impl ThreadKey {
    fn mint() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        thread_local! {
            static KEY: ThreadKey = {
                let raw = NEXT.fetch_add(1, Ordering::Relaxed);
                ThreadKey(std::num::NonZeroU64::new(raw).expect("thread-key mint starts at 1"))
            };
        }
        KEY.with(|key| *key)
    }

    fn load(
        cell: &std::sync::atomic::AtomicU64,
        order: std::sync::atomic::Ordering,
    ) -> Option<Self> {
        std::num::NonZeroU64::new(cell.load(order)).map(Self)
    }

    fn store(
        cell: &std::sync::atomic::AtomicU64,
        key: Option<Self>,
        order: std::sync::atomic::Ordering,
    ) {
        cell.store(key.map_or(0, |ThreadKey(n)| n.get()), order);
    }
}

/// Clears the owner mark when the write closure exits — normally, by
/// error, or by unwind — so the next `write` on this thread proceeds.
struct WriterThreadReset<'a>(&'a std::sync::atomic::AtomicU64);

/// One admitted LMDB read lease: executes prepared queries and exports
/// relations. Handed to [`Db::read`] closures. The transaction borrows
/// the environment; the lease is invalidated by Rust lifetime when the
/// callback returns. `!Send + !Sync` — a read lease does not cross
/// threads.
///
/// ```compile_fail
/// fn require_send<T: Send>() {}
/// require_send::<bumbledb::ReadInstance<'static, ()>>();
/// ```
///
/// ```compile_fail
/// fn require_sync<T: Sync>() {}
/// require_sync::<bumbledb::ReadInstance<'static, ()>>();
/// ```
///
/// ```compile_fail
/// fn require_insert(instance: &mut bumbledb::ReadInstance<'_, ()>) {
///     let _ = instance.insert;
/// }
/// ```
pub struct ReadInstance<'txn, S> {
    pub(super) core: instance::InstanceCore<crate::image::LmdbSource<'txn>, S>,
    thread_bound: PhantomData<Rc<()>>,
}

impl<'txn, S> ReadInstance<'txn, S> {
    /// The lease's read transaction (reader: the staleness signal —
    /// [`crate::PreparedQuery::staleness`] takes the instance directly
    /// rather than routing through a wrapper method).
    pub(crate) fn txn(&self) -> &ReadTxn<'_> {
        self.core.source.txn()
    }

    pub(crate) fn cache(&self) -> &'txn ImageCache {
        self.core.source.cache()
    }

    /// The lease's witnessed generation: the storage tx id read from
    /// `_meta` **inside this read transaction** — the race-closing rule of
    /// `docs/architecture/50-storage.md` — memoized on the read
    /// transaction. A scoped read that wants its generation reads it
    /// here, never through a second transaction (the FFI bridge rides it
    /// on the open reply).
    ///
    /// # Errors
    ///
    /// `Corruption` on a missing or malformed tx id.
    pub fn generation(&self) -> Result<crate::GenerationId> {
        self.txn().generation()
    }

    /// Runs `body` with a pooled point-read scratch set, restoring it
    /// afterward — capacity included — success or error. The one scratch
    /// discipline of every read-instance point read (R15).
    fn with_scratch<R>(&self, body: impl FnOnce(&mut ReadScratch) -> Result<R>) -> Result<R> {
        self.core.with_scratch(body)
    }
}

/// One write transaction: an in-memory write delta over a read view. Operations
/// are in-memory set arithmetic — order is semantically irrelevant, and
/// `delete(old); insert(new)` in either order is the blessed mutation
/// idiom. Handed to [`Db::write`] closures; offers no queries — point
/// reads only ([`WriteTx::contains`] / [`WriteTx::get`] /
/// [`WriteTx::get_dyn`]), which observe the final-state view the judgment
/// phase will judge. No prepared-query or [`ReadInstance`] is reachable from
/// here (`docs/architecture/70-api.md`: full queries in write transactions
/// stay unrepresentable). Carries the handle's schema typestate `S`:
/// typed operations bound `F: Fact<'_, Schema = S>`.
pub struct WriteTx<'a, S> {
    mutation: mutation_core::MutationCore<mutation_core::StoreMutation<'a>, S>,
}

impl<'a, S> WriteTx<'a, S> {
    fn into_store(self) -> (ReadTxn<'a>, WriteDelta<'a>) {
        self.mutation.into_store()
    }

    fn poisoned(&self) -> Option<&crate::error::Error> {
        self.mutation.poisoned()
    }

    #[cfg(test)]
    pub(crate) fn delta(&self) -> &WriteDelta<'a> {
        &self.mutation.backend.delta
    }
}

/// `schema!` expansion plumbing: the generated `Fact` impls call these.
/// Not API — no stability promises; nothing here is reachable from the
/// documented surface.
#[doc(hidden)]
pub mod plumbing;

impl<S> codec_seal::Sealed for ReadInstance<'_, S> {}
impl<S> codec_seal::Sealed for WriteTx<'_, S> {}

impl<S> CodecRead<S> for ReadInstance<'_, S> {
    fn schema(&self) -> &Schema {
        self.core.schema.as_ref()
    }

    fn lookup_str(&self, value: &str) -> Result<Option<InternId>> {
        self.core.source.catalog().dict_lookup(value.as_bytes())
    }

    fn resolve_str(&self, id: InternId) -> Result<&str> {
        plumbing::resolve_string(self, id)
    }
}

impl<S> CodecRead<S> for WriteTx<'_, S> {
    fn schema(&self) -> &Schema {
        self.mutation.schema()
    }

    fn lookup_str(&self, value: &str) -> Result<Option<InternId>> {
        CodecRead::lookup_str(&self.mutation, value)
    }

    fn resolve_str(&self, id: InternId) -> Result<&str> {
        CodecRead::resolve_str(&self.mutation, id)
    }
}

impl<S> CodecWrite<S> for WriteTx<'_, S> {
    fn intern_str(&mut self, value: &str) -> Result<InternId> {
        CodecWrite::intern_str(&mut self.mutation, value)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod append_tests;

#[cfg(all(test, feature = "trace"))]
mod trace_tests;
