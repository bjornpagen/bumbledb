# Proposal: First-Class C++26 SDK for Bumbledb

**Status:** Implementation proposal — rev 2 (2026-08-08: all engine seams verified against source; §38 reflection spike executed and PASSED on the pinned GCC 16.1; TRY-macro spelling removed; panic/FFI policy added; error taxonomy completed; cpp-starter template vendoring specified)
**Layout note (post-implementation):** file layout as built diverges from the sketches below — the shipped tree uses one `.cc` extension for every source file (no `.cppm`), one named module `bumbledb` with internals as partitions (no dotted module names), and the `meta/` zone is retired (reflective code is ordinary partition code beside its callers). `cpp/AGENTS.md` is normative for layout; the §-numbered semantics in this record remain the design authority.
**Audience:** Coding agent implementing the C++ SDK and its Rust FFI bridge
**Scope:** `cpp/` SDK and bridge only; no Bumbledb engine rewrite
**Host language:** Strict project C++26 dialect
**Engine:** Existing Rust Bumbledb engine
**Primary design criterion:** The C++ SDK must express the same structural theory as the TypeScript SDK while preserving Bumbledb’s existing Rust engine as the sole runtime semantic authority.

---

# 1. Executive decision

Bumbledb SHALL remain implemented in Rust.

A new C++26 SDK SHALL be added as a downstream host-language frontend and runtime binding.

The architecture SHALL be:

```text
                         LEAN SPEC
                            │
                            │ defines/checks semantics
                            ▼
                     RUST BUMBLEDB ENGINE
                 storage / validation / planning
                   execution / transactions
                            │
               canonical descriptors + IR
                  ┌─────────┴─────────┐
                  │                   │
                  ▼                   ▼
            TYPESCRIPT SDK       C++26 SDK
              structural         structural +
               frontend          reflective frontend
```

The C++ SDK SHALL NOT implement an independent database engine, independent schema validator, independent query planner, or independent transaction system.

Its compile-time responsibility is:

```text
ordinary C++ declarations
        ↓
C++26 reflection
        ↓
structural Bumbledb relations
        ↓
consteval statement/query elaboration
        ↓
named SchemaSpec / plain Query IR
        ↓
narrow C ABI
```

Its runtime responsibility is:

```text
RAII/value C++ API
        ↓
narrow C ABI
        ↓
existing Rust dynamic API
```

The Rust engine remains authoritative whenever runtime semantic validation is required.

This separation matches the repository's existing design. `SchemaSpec` is explicitly described in the Rust source as “the bindings contract”: named plain data provided by a foreign host, lowered by the shared Rust implementation to the same descriptor used elsewhere. The source explicitly states that bindings may marshal it however they choose and that the engine owns the canonical lowering.

---

# 2. Why this architecture fits Bumbledb

The C++ SDK is not intended to invent a new Bumbledb theory.

The existing TypeScript cookbook already demonstrates the desired semantic model. Relation declarations contain only structural representation information; semantic domain classes are derived from the statement list itself. Containments, mirrors and other paired faces induce equivalence classes through union-find, and those classes type query-variable reuse. There is deliberately no separate domain declaration surface.

The C++ SDK SHALL preserve this property.

Given:

```cpp
struct ServiceRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    std::string name;
};

struct OutageRow {
    std::uint64_t service;
    bdb::interval<std::int64_t> window;
};
```

the SDK SHALL NOT require:

```cpp
struct ServiceId;
```

or:

```cpp
using ServiceId = ...;
```

or:

```cpp
implements_domain<ServiceId>
```

or any other nominal wrapper merely to distinguish `Service.id` from another `u64`.

Instead:

```cpp
bdb::contained(
    bdb::on(Outage.service),
    bdb::on(Service.id)
)
```

is itself the semantic fact that unifies those two coordinates.

Conceptually:

```text
physical representation

Service.id      : u64
Outage.service  : u64


schema elaboration

Outage.service ─────┐
                    ├── domain class "Service.id"
Service.id [fresh] ─┘
```

The statement is the typing.

The C++ compile-time frontend SHALL reproduce that judgment for authoring and query checking, while the resulting domain-class metadata SHALL NOT become a runtime wrapper around the physical `u64`.

This is the core philosophical requirement of the SDK.

---

# 3. Existing engine seams that SHALL be reused

The implementation MUST begin from the public dynamic surface already present in Bumbledb.

The engine already has a complete runtime-schema path based on:

```rust
Db<SchemaDescriptor>
```

and the Node bridge already uses that form for runtime-built schemas.

The bridge SHALL use the existing engine operations rather than add C++-specific engine variants.

The important existing seams are:

| Requirement                 | Existing engine seam                      |
| --------------------------- | ----------------------------------------- |
| Foreign schema construction | `SchemaSpec` → `SchemaSpec::descriptor()` |
| Runtime theory              | `Db<SchemaDescriptor>` (via `impl Theory for SchemaDescriptor`) |
| Store lifecycle             | `Db::create` / `Db::open` / `Db::ephemeral` |
| Query/program preparation   | `Db::prepare` (borrows the `Db`; result is `!Sync`, `&mut` per execution) |
| Read snapshot               | `Db::read` (closure returns `Result<R>`; snapshot is higher-ranked, cannot escape) |
| Unconditional write         | `Db::write` |
| Snapshot-derived write      | `Db::write_from` / `Db::write_from_witness` |
| Foreign-host insert/delete  | `WriteTx::insert_dyn` / `delete_dyn` (`&[Value]`, declaration order) |
| Foreign-host point reads    | `contains_dyn` / `get_dyn` on both `Snapshot` and `WriteTx`, plus pooled `get_dyn_into` reuse variants |
| Foreign export              | dynamic `Snapshot::scan` (iterator borrows the snapshot) |
| Foreign bulk import         | `Db::bulk_load_dyn` (atomic 4096-row chunks) |
| Fresh identity              | `Db::fresh_field` (resolve once per relation) + `WriteTx::alloc_at` (mint per row) |
| Query result storage        | `Answers` (flat cells + separate text/blob heaps; `clear()` retains capacity) |
| Runtime query parameters    | `BindValue` / `ParamArg` (public variants: `Scalar`, `Set`; Allen masks travel as scalars) |

All seams in this table were verified against the engine source (2026-08-08).
Three engine facts contradict older prose and are binding on this document:
`Snapshot::generation()` DOES exist (`api/db.rs`, added for FFI bridges) even
though `70-api.md` claims otherwise; the engine refuses nested writes with an
assertion (a panic), not a typed error; and `Answers::get` panics on
out-of-range access. The bridge consequences are specified in §17, §22 and
§30.

The architecture documentation explicitly identifies the dynamic write lane as `insert_dyn` / `delete_dyn` and the dynamic bulk lane as `bulk_load_dyn`; these are intended for FFI/ETL use rather than being second-class hacks.

No generated Rust `Fact` type is required for the C++ SDK.

---

# 4. Repository layout

The proposed repository shape is:

```text
bumbledb/
├── crates/
│   ├── bumbledb/
│   ├── bumbledb-theory/
│   ├── ...
│
├── ts/
│   ├── crate/
│   └── ...
│
└── cpp/
    ├── AGENTS.md
    ├── .clang-tidy
    ├── .clang-format
    │
    ├── bridge/
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs
    │       ├── db.rs
    │       ├── schema.rs
    │       ├── query.rs
    │       ├── value.rs
    │       ├── answers.rs
    │       └── error.rs
    │
    ├── foreign/
    │   ├── bumbledb_c.h
    │   └── bridge.cppm
    │
    ├── src/
    │   ├── bumbledb.cppm
    │   ├── bumbledb.types.cppm
    │   ├── bumbledb.schema.cppm
    │   ├── bumbledb.query.cppm
    │   ├── bumbledb.db.cppm
    │   └── bumbledb.answers.cppm
    │
    ├── meta/
    │   ├── bumbledb.meta.relation.cppm
    │   ├── bumbledb.meta.schema.cppm
    │   ├── bumbledb.meta.query.cppm
    │   └── bumbledb.meta.row.cppm
    │
    ├── tests/
    │   ├── cookbook/
    │   ├── compile_fail/
    │   ├── bridge/
    │   └── runtime/
    │
    ├── CMakeLists.txt
    └── CMakePresets.json
```

`cpp/` mirrors the `ts/` precedent completely: a sibling subdirectory carrying
its own full toolchain and normative dialect infrastructure, cloned from the
existing cpp-starter template (github.com/bjornpagen/cpp-starter). `AGENTS.md`
(the normative dialect), the clang-tidy configuration with its query-based
custom checks, the format configuration, the preset personalities
(`dev` / `release` / `asan-ubsan` / `tsan` / `lint`) and the exact-tuple
configure gate all come from that template. §32 records the template-derived
toolchain facts that bind this SDK.

The relationship is fork-and-own: once cloned, `cpp/` is the living copy and
evolves with Bumbledb. There is no sync obligation back to the template.

`cpp/bridge` SHOULD be a standalone downstream Rust crate rather than a member of the root Rust workspace.

This follows the existing Node binding precedent: `ts/crate` is deliberately excluded from the engine workspace so binding-specific dependencies do not contaminate the engine workspace.

`cpp/bridge` SHOULD therefore contain its own:

```toml
[workspace]
```

and the root workspace SHOULD explicitly exclude it.

The bridge SHOULD depend directly on:

```toml
bumbledb = { path = "../../crates/bumbledb" }
```

and SHOULD initially export:

```toml
crate-type = ["staticlib", "cdylib"]
```

The static library is the preferred native application path.

The shared library exists for downstream packaging and testing.

---

# 5. Strict C++26 dialect requirements

Everything in:

```text
cpp/src/
cpp/meta/
cpp/tests/
```

MUST obey the project C++26 dialect.

Everything in:

```text
cpp/foreign/
```

is an explicit foreign zone and may use C headers and raw ABI machinery.

The SDK MUST use the pinned project compiler/toolchain policy, including GCC 16.x as the authoritative frontend and the project's reflection configuration.

The application-facing SDK MUST obey all existing dialect rules, including the following consequences.

No application-facing preprocessor use is permitted.

No project headers are permitted. Project code uses named modules.

No exceptions are permitted.

No RTTI is permitted.

No inheritance or virtual dispatch is permitted.

No `shared_ptr`, `weak_ptr`, `enable_shared_from_this`, or shared ownership model is permitted.

No application-facing `new`, `delete`, `malloc`, `free`, raw allocation, or owning raw pointers are permitted.

No `std::function` is permitted.

No direct coroutine syntax is permitted.

No `std::async`, futures/promises, arbitrary thread creation, or detached work is permitted.

No raw pointers SHALL appear in application-facing APIs.

Recoverable runtime errors SHALL use:

```cpp
std::expected<T, E>
```

There is no `TRY(...)` macro and there never will be one: the dialect bans the
preprocessor and C++26 has no try-operator. Failure composition is monadic
(`and_then` / `transform` / `or_else`) or an explicit early return on an
`expected` condition. Every example in this document is spelled accordingly.

Closed alternatives SHALL use `std::variant` or proper enum/closed-value types.

Optional values SHALL use `std::optional`.

Borrowed sequences SHALL use `std::span`.

Borrowed strings SHALL use `std::string_view`.

Resource-owning objects SHALL be move-only RAII classes.

Compile-time invalidity SHALL be rejected during consteval/concept elaboration whenever sufficient information is available.

The C bridge is the one location where C pointers, `extern "C"`, raw buffers, generated C declarations and unavoidable unsafe mechanics are legal.

---

# 6. Public C++ relation model

A Bumbledb ordinary relation SHALL originate from an ordinary C++ product type.

Target spelling:

```cpp
struct ServiceRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    std::string name;
};

struct OutageRow {
    std::uint64_t service;
    bdb::interval<std::int64_t> window;
};

inline constexpr auto Service =
    bdb::relation<"Service", ServiceRow>;

inline constexpr auto Outage =
    bdb::relation<"Outage", OutageRow>;
```

`relation<...>` SHALL use C++26 reflection to inspect the row type.

The SDK SHALL derive from the actual declaration:

```text
relation name
field declaration order
field names
physical C++ types
Bumbledb structural ValueType
fresh annotations
```

Reflection metadata SHALL be the sole field-source-of-truth.

The user SHALL NOT provide a parallel field list.

This is forbidden:

```cpp
bdb::relation(
    "Service",
    bdb::field("id", bdb::u64),
    bdb::field("name", bdb::str)
);
```

because the C++ struct already states the representation.

The relation descriptor type SHOULD expose reflected coordinate values:

```cpp
Service.id
Service.name

Outage.service
Outage.window
```

These are not runtime field values.

They are compile-time semantic coordinates.

Conceptually:

```cpp
Outage.service
```

represents:

```text
relation = Outage
field    = service
kind     = u64
ordinal  = 0
```

The implementation SHOULD synthesize the coordinate facade from the reflected row declaration.

The reflection/splicing mechanism has been verified against the pinned GCC
(§38, spike complete): named facade members synthesize cleanly via
`define_aggregate`, so no fallback representation is needed. Macro-generated
field facades remain forbidden.

---

# 7. Supported row representation vocabulary

Initial C++ physical types SHALL map to Bumbledb's existing value roster.

The Rust `Value` sum currently distinguishes `Bool`, `U64`, `I64`, UTF-8 string bytes, fixed bytes, checked U64/I64 intervals and Allen masks.

The C++ mapping SHALL therefore begin as:

```text
bool
    → Bool

std::uint64_t
    → U64

std::int64_t
    → I64

std::string
    → String

std::array<std::byte, N>
or bdb::bytes<N>
    → FixedBytes<N>

bdb::interval<std::uint64_t>
    → IntervalU64

bdb::interval<std::int64_t>
    → IntervalI64

bdb::interval<T, Width>
    → corresponding fixed-width interval ValueType

bdb::allen_mask
    → query parameter / literal Allen mask
```

Unsupported reflected C++ types MUST fail at compile time.

The SDK SHALL NOT silently serialize arbitrary structs, numeric widths, enums, pointers, references or standard-library containers into Bumbledb values.

Bumbledb's structural value vocabulary is closed.

---

# 8. Closed relations

Closed Bumbledb relations MUST remain relational.

They MUST NOT be replaced conceptually by a C++ `enum class`.

However, the SDK MAY synthesize a host closed-handle enum/value type as a projection of the relation, exactly as the current Rust host does: the engine vocabulary remains relational while the host obtains exhaustive pattern matching. The existing architecture explicitly treats the Rust enum as an emitted host projection rather than the engine representation.

Desired user surface:

```cpp
inline constexpr auto Kind =
    bdb::closed<"Kind">(
        "Deterministic",
        "CustomOperator"
    );
```

For payload-bearing vocabularies:

```cpp
struct KindPayload {
    bool mastered;
    std::uint64_t rank;
};

inline constexpr auto Kind =
    bdb::closed<"Kind", KindPayload>(
        bdb::member<"DirectPass">({
            .mastered = true,
            .rank = 30,
        }),

        bdb::member<"JudgedPass">({
            .mastered = true,
            .rank = 20,
        }),

        bdb::member<"Failed">({
            .mastered = false,
            .rank = 10,
        })
    );
```

The resulting relation facade SHOULD expose:

```cpp
Kind.id
Kind.mastered
Kind.rank

Kind.DirectPass
Kind.JudgedPass
Kind.Failed

Kind.axioms.DirectPass.rank
```

The closed relation itself SHALL remain usable in schema statements and query atoms.

The host handle projection SHALL not replace `Kind` as a relation.

The bridge SHALL populate the existing `SchemaSpec` closed-relation metadata required by the Rust binding contract. Any internal host-newtype labels required by the current `SchemaSpec` implementation are implementation data only; they SHALL NOT create user-visible nominal domain wrappers. `SchemaSpec` currently carries host newtype names for closed-handle resolution and drops those names during descriptor lowering.

---

# 9. Schema statement algebra

The C++ statement vocabulary SHALL mirror the TypeScript cookbook rather than inventing SQL-like sugar.

Representative target:

```cpp
inline constexpr auto Uptime =
    bdb::schema<"Uptime">(
        Service,
        Outage,

        bdb::contained(
            bdb::on(Outage.service),
            bdb::on(Service.id)
        ),

        bdb::key(
            Outage.service,
            Outage.window
        )
    );
```

The primary constructors are conceptually:

```cpp
bdb::key(...)
bdb::contained(...)
bdb::mirrors(...)
bdb::capacity(...)
bdb::on(...)
bdb::weigh(...)
bdb::within(...)
bdb::ref(...)
bdb::duration(...)
```

The C++ surface SHALL preserve the positional semantics of the existing theory.

For example, capacity is not to be redesigned as a generic SQL `group_by().sum()` API.

The cookbook defines it as a particular law with target, weight, bounds/window and source roles.

The target C++ spelling therefore remains structurally close:

```cpp
bdb::capacity(
    bdb::on(Pool.id),

    bdb::weigh(
        Device.watts
    ),

    bdb::within(
        std::uint64_t{0},
        bdb::ref(Pool.supply)
    ),

    bdb::on(Device.pool)
)
```

The frontend SHALL not duplicate semantic policy already centralized in Rust.

At compile time, C++ MAY reject errors that are structurally decidable from the C++ theory.

At runtime/create time, the named schema specification SHALL still be handed to Rust and lowered by `SchemaSpec::descriptor()` so canonical name resolution, canonical-utterance rules and the engine's semantic schema validation remain authoritative. `SchemaSpec` explicitly centralizes those responsibilities today.

---

# 10. Compile-time field classes

The C++ schema object SHALL compute the cookbook's class laws during constant evaluation.

Given all projected paired faces from:

```text
contained
mirrors
capacity target/source pairing
selected faces
```

the C++ frontend SHALL run union-find over field coordinates.

Fresh fields are generators.

Closed-relation IDs are generators.

Generator-less classes receive deterministic coordinate identities using the same declaration-order principle as the TypeScript SDK.

Unifying two different generators SHALL be a compile-time error.

A field that participates in no law SHALL remain bare.

Bare fields SHALL only query-unify with other compatible bare fields according to the same host rule used by the TypeScript SDK.

This compile-time class metadata exists to type the C++ query language.

It SHALL NOT become part of the physical field representation.

It SHALL NOT alter the engine fingerprint.

The TypeScript cookbook explicitly states both that the laws compute these classes and that class identity does not enter the fingerprint.

---

# 11. Query API

The C++ query language SHALL follow the existing Bumbledb query algebra closely.

It SHALL NOT expose the low-level IR builder as the everyday user surface.

Target:

```cpp
inline constexpr auto DownAt =
    bdb::query(Uptime)
        .rule([](auto r) consteval {
            auto vars =
                r.vars(Outage);

            auto service =
                vars.service;

            auto window =
                vars.window;

            return r
                .match(
                    Outage,
                    {
                        .service = service,
                        .window = window,
                    }
                )
                .where(
                    bdb::point_in(
                        r.param<"t">(),
                        window
                    )
                )
                .find({
                    .service = service,
                });
        });
```

This is intentionally analogous to the TypeScript cookbook's:

```text
v(Relation)
match
where
find
param
```

rather than being a different query language. The cookbook's variables are minted per relation column and reused to express joins, with their semantic legality determined by the schema's law classes.

`r.vars(Relation)` SHALL return a synthesized product whose members correspond to the relation fields.

Variable binding is member access, deliberately and only:

```cpp
auto vars = r.vars(Outage);

vars.service
vars.window
```

The TypeScript SDK deliberately supports exactly one binding style, and its
destructuring is NAMED (`const { service, window } = v(Outage)`). C++
structured bindings are positional — a different, weaker thing that hides the
field name at the binding site. They SHALL NOT be part of the supported
surface: explicit is better than implicit, and the field name appears at
every use.

Each variable SHALL carry a compile-time domain-class identity derived from the schema.

Reusing one variable against a field in another semantic class SHALL fail during constant evaluation.

The physical type alone is insufficient.

Two unrelated `u64` coordinates are not query-compatible merely because both are `uint64_t`.

The query frontend SHALL lower to ordinary Bumbledb query/program IR.

Rust SHALL still run the existing IR validator during `prepare`.

Compile-time validation supplements that boundary; it does not replace it.

---

# 12. Query result typing

`.find(...)` SHALL determine a compile-time answer row type.

Example:

```cpp
.find({
    .service = service,
    .window = window,
})
```

SHOULD yield a synthesized row representation equivalent to:

```cpp
struct AnswerRow {
    std::uint64_t service;
    bdb::interval<std::int64_t> window;
};
```

The user SHALL NOT manually declare this type.

The query object SHALL carry enough compile-time information for:

```cpp
bdb::Prepared<DownAt>
```

and:

```cpp
bdb::Answers<DownAt>
```

to know the query parameter and result shape.

Aggregate synthesis is verified on the pinned GCC (§38): named answer-row
synthesis works via consteval-block `define_aggregate`, so named structural
answers corresponding to `.find` are the committed target. No tuple fallback
is needed.

---

# 13. Schema and query marshalling to Rust

C++ SHALL NOT invent JSON, protobuf, serde, or a custom persisted wire protocol for schema/query crossing.

The boundary exists only within the current process.

Schema creation SHALL produce compile-time C++ data whose layout can be presented to the C bridge as borrowed slices.

Conceptually:

```c
typedef struct {
    char const* data;
    size_t size;
} bdb_string_view;

typedef struct {
    bdb_string_view name;
    bdb_value_type value_type;
    bdb_string_view internal_newtype;
    bool fresh;
} bdb_field_spec_view;

typedef struct {
    bdb_string_view name;

    bdb_field_spec_view const* fields;
    size_t field_count;

    bdb_closed_spec_view const* closed;
} bdb_relation_spec_view;

typedef struct {
    bdb_relation_spec_view const* relations;
    size_t relation_count;

    bdb_statement_spec_view const* statements;
    size_t statement_count;
} bdb_schema_spec_view;
```

The bridge MUST immediately copy these views into Rust-owned `SchemaSpec`.

No borrowed C++ memory survives `bdb_db_create` or `bdb_db_open`.

Rust then calls:

```rust
SchemaSpec::descriptor()
```

and passes the descriptor to:

```rust
Db::<SchemaDescriptor>::create(...)
```

or:

```rust
Db::<SchemaDescriptor>::open(...)
```

The C++ frontend MUST NOT reproduce the canonical Rust lowering as the runtime source of truth.

Lowering parity with the TypeScript SDK is binding: the C++ frontend lowers
DECLARED statements only — the engine materializes fresh-implied keys and
closed auto-keys itself — and the union-find class names cross only as each
field's `newtype` slot (dropped at descriptor lowering, never fingerprinted).
The C++ class-naming discipline SHALL mirror the TypeScript SDK's
(generator-first, else least member coordinate in relation-declaration ×
field-declaration order) so spec coherence checks and diagnostics agree
across hosts.

Likewise, a constexpr C++ query/program SHALL expose a static C-compatible IR view.

At `prepare`, Rust copies that view into the existing Rust `Program` representation and calls `Db::prepare`.

The current engine's prepare path performs validation, normalization, statistics reads and planning once, returning a reusable `PreparedQuery`.

---

# 14. C ABI philosophy

The C ABI SHALL be intentionally boring.

It exists only as the boundary between the safe-ish C++ dialect and Rust.

The ABI SHALL use:

```text
opaque handles
plain integers
tagged POD structs
borrowed pointer+length views
callback function pointers
explicit destroy functions
```

The ABI SHALL NOT expose:

```text
Rust references
Rust enums with unspecified ABI
Rust Vec/String
C++ standard-library types
templates
exceptions
vtable objects
shared ownership
```

All ABI structs SHALL use explicit fixed-width integer fields and `#[repr(C)]`.

All exported Rust functions SHALL use `extern "C"`.

A generated pure-C header SHOULD be produced from the Rust ABI declarations using a pinned `cbindgen`-style tool or equivalent.

The generated header belongs to `cpp/foreign/`, not the C++ dialect source.

The generated header MAY use the preprocessor because `foreign/` is explicitly the compatibility zone.

The public C++ module SHALL import/wrap it so no C declarations leak upward.

---

# 15. Database ownership

The bridge SHALL expose an opaque database handle:

```c
typedef struct bdb_db bdb_db;
```

The C++ wrapper:

```cpp
class Db {
    foreign::db_handle handle_;

public:
    Db(Db const&) = delete;
    auto operator=(Db const&) -> Db& = delete;

    Db(Db&&) noexcept;
    auto operator=(Db&&) noexcept -> Db&;

    ~Db();

    // ...
};
```

No shared ownership SHALL exist in the C++ API.

`Db` is an owning resource/capability.

The ABI SHALL expose all three store constructors the engine provides:
`create`, `open`, and `ephemeral`.

The bridge MAY internally use Rust `Arc<Db<SchemaDescriptor>>` where necessary to support prepared-query ownership, because that shared ownership remains below the foreign boundary and is not part of the application semantic model.

---

# 16. Read snapshots: preserve Rust's lexical model

The C++ SDK SHALL NOT copy the Node bridge's worker-thread snapshot protocol.

The Rust API already expresses snapshots lexically:

```rust
Db::read(|snapshot| ...)
```

`Db::read` constructs one snapshot, invokes the closure synchronously, then parks or drops that snapshot before returning.

C++ can model this directly through synchronous C callbacks.

Proposed ABI:

```c
typedef struct bdb_snapshot_ref bdb_snapshot_ref;

typedef enum {
    BDB_CALLBACK_OK,
    BDB_CALLBACK_ABORT
} bdb_callback_control;

typedef bdb_callback_control (*bdb_read_callback)(
    void* context,
    bdb_snapshot_ref const* snapshot
);

bdb_status bdb_db_read(
    bdb_db const* db,
    bdb_read_callback callback,
    void* context
);
```

The Rust implementation performs:

```rust
db.read(|snapshot| {
    let borrowed =
        SnapshotRef::from(snapshot);

    let control =
        callback(context, &raw const borrowed);

    match control {
        Ok => Ok(()),
        Abort => Err(binding_abort()),
    }
})
```

The C++ trampoline constructs a non-owning `Snapshot` wrapper valid only during the callback:

```cpp
auto result =
    db.read([&](bdb::Snapshot& snap) {
        // all reads here
        return ...;
    });
```

`Snapshot` SHALL be non-copyable and non-movable.

It SHALL have no owning constructor.

Its underlying ABI handle SHALL never be exposed.

Returning a `Snapshot` by value SHALL be impossible.

The SDK's semantic rule is:

> A snapshot is a lexical borrowed capability whose lifetime is exactly the `Db::read` callback.

The dialect's ordinary no-escaping-borrow policy applies.

`Snapshot::generation()` exists on the Rust side (added for FFI bridges); the
C++ SDK still SHALL NOT expose a raw generation number. The bridge SHALL
invalidate the `bdb_snapshot_ref` when the callback returns (debug poisoning
at minimum), so a stashed ref cannot be replayed after `Db::read` returns.

---

# 17. Writes: preserve Rust's lexical model

The write boundary SHALL use the same technique.

Rust's existing `Db::write` accepts a closure over `&mut WriteTx`, and any error returned by that closure aborts the entire delta before it commits. The implementation holds the writer discipline, constructs the delta, invokes the callback and only then passes the completed delta to commit.

Proposed ABI:

```c
typedef struct bdb_tx_ref bdb_tx_ref;

typedef bdb_callback_control (*bdb_write_callback)(
    void* context,
    bdb_tx_ref* transaction
);

bdb_status bdb_db_write(
    bdb_db* db,
    bdb_write_callback callback,
    void* context
);
```

C++:

```cpp
auto result =
    db.write([&](bdb::WriteTx& tx) {
        return tx.alloc(Service.id)
            .and_then([&](std::uint64_t id) {
                return tx.insert(
                    ServiceRow{
                        .id = id,
                        .name = "search",
                    }
                );
            })
            .transform([](auto&&) {
                return bdb::commit();
            });
    });
```

`WriteTx` SHALL be lexical, non-copyable and non-movable.

No transaction handle SHALL outlive the callback.

The bridge SHALL not create a transaction object that the C++ caller separately commits later.

Rust's existing lexical commit boundary is superior and should remain visible in C++.

The engine refuses nested writes with an assertion (a panic), not a typed
error. The bridge SHALL therefore refuse re-entrant `write` / `write_from` /
`bulk_load` at the bridge layer with a typed error — a handle-level in-write
flag — before the engine's assertion can fire, because a Rust panic crossing
the C boundary is undefined behavior (§30).

---

# 18. Snapshot-derived writes

The SDK SHALL expose the existing `write_from` semantics.

Rust already accepts:

```rust
Db::write_from(&snapshot, ...)
```

and checks the snapshot's generation inside the writer critical section before the write closure executes. A moved generation returns `GenerationMoved`; retry is host policy.

C++ target:

```cpp
auto update =
    db.read([&](bdb::Snapshot& snap) {
        return snap.execute(prepared, params)
            .and_then([&](auto source) {
                return db.write_from(
                    snap,

                    [&](bdb::WriteTx& tx) {
                        // build delta from source

                        return bdb::commit();
                    }
                );
            });
    });
```

The implementation MAY synchronously re-enter Rust from the C++ read callback.

No snapshot is moved across a thread.

No raw generation number is exposed.

No reified witness is necessary for the ordinary C++ path.

This nesting is sound for exactly one reason, and the bridge SHALL record it
as its SAFETY argument: the C++ read callback executes synchronously inside
the Rust `Db::read` closure frame on the same thread, so the `&Snapshot`
behind `bdb_snapshot_ref` is alive for the entire nested call. Both `read`
and `write_from` take `&self`, and this nesting is proven in-tree
(`bumbledb-bench` witness tests; `bumbledb-query` cookbook). Audit findings
018/021 document real undefined behavior in the Node bridge from fabricating
a `&'static Snapshot` for this same call; the C++ bridge avoids that entire
class by never storing the snapshot reference — it only forwards the
still-live callback argument.

The existing `write_from_witness` path is primarily useful for boundaries, like Node's, that cannot preserve the lexical borrow; the Rust source explicitly documents it as the FFI lane for such callers.

The C++ SDK should use the stronger direct `Snapshot` path whenever practical.

Nested writes from inside another write callback MUST remain forbidden because the engine write API is explicitly non-reentrant.

---

# 19. C++ write outcome algebra

No exceptions SHALL cross or exist at the C++ API.

The SDK SHOULD represent write callback intent explicitly.

Recommended conceptual types:

```cpp
template<class T>
struct Commit {
    T value;
};

template<class T>
struct Abandon {
    T value;
};

template<class T, class A>
using WriteDecision =
    std::variant<
        Commit<T>,
        Abandon<A>
    >;
```

The outer operation:

```cpp
template<class T, class A>
using WriteOutcome =
    std::variant<
        Committed<T>,
        Abandoned<A>
    >;
```

and:

```cpp
auto Db::write(...)
    -> std::expected<
        WriteOutcome<...>,
        bdb::Error
    >;
```

Domain-level abandonment is data, not an exception and not an engine error.

Engine failure or commit rejection is the `unexpected` path.

If user callback code itself needs a typed application error, an additional generic host-error wrapper MAY be introduced, but the API MUST not collapse abandonment, host failure and engine failure into one boolean or string.

---

# 20. Prepared queries

The prepared-query bridge is the one place where a small amount of deliberate unsafe Rust is expected.

`PreparedQuery` borrows the preparing engine's schema and contains mutable reusable scratch. It is intentionally `!Sync` and reusable across snapshots of the same environment.

The existing Node bridge solves foreign ownership by keeping an `Arc<Engine>` in the same owning object and erasing the prepared query's borrow lifetime to `'static`, with drop order ensuring that the prepared query dies before its owning engine reference.

The C bridge MAY initially reuse that exact ownership argument.

Proposed Rust representation:

```rust
struct PreparedHandle {
    prepared:
        PreparedQuery<'static, SchemaDescriptor>,

    db:
        Arc<Db<SchemaDescriptor>>,
}
```

`prepared` MUST be declared before `db` so reverse drop order destroys the borrow before its owner.

Every unsafe lifetime erasure MUST have a local `SAFETY` proof explaining:

```text
stable engine allocation
engine kept alive by owning Arc
prepared borrow points only into that engine
prepared dropped before Arc
no concurrent mutable access
```

Longer-term, an engine-level `OwnedPreparedQuery` MAY centralize this proof for all foreign bindings, but this is not required for the first SDK.

C++:

```cpp
template<auto Query>
class Prepared {
    foreign::prepared_handle handle_;

public:
    Prepared(Prepared const&) = delete;

    auto operator=(Prepared const&)
        -> Prepared& = delete;

    Prepared(Prepared&&) noexcept;

    auto operator=(Prepared&&) noexcept
        -> Prepared&;

    ~Prepared();
};
```

Prepared queries are move-only.

Concurrent execution through one prepared object is outside the dialect's permitted model.

The SDK MUST NOT introduce shared prepared-query ownership merely to support arbitrary concurrent calls.

---

# 21. Query parameters

The compile-time query object SHALL derive its parameter schema.

Given:

```cpp
r.param<"t">()
```

whose use against an interval point determines an `i64` domain, the final query SHALL know:

```text
parameter name
parameter ordinal
scalar/set/mask shape
Bumbledb structural value type
point-domain status
```

The engine records the scalar/set/mask contract in prepared-query bind specs.
The public runtime surface is narrower than that contract: `ParamArg` has
exactly two variants (`Scalar`, `Set`), and an Allen mask travels as
`Scalar(BindValue::AllenMask)`. The C ABI SHALL mirror the public shape, not
the private spec.

`BindValue` intervals are raw `(lo, hi)` pairs, while stored `Value` intervals
are checked (`start < end` enforced at construction). `bdb::interval` SHALL
therefore have two construction lanes, both checked: a consteval factory for
literals (an invalid constant interval is a compile error) and a runtime
factory returning `std::expected`, so the bridge can never present an
unrepresentable interval to the engine.

C++ call target:

```cpp
auto rows =
    snap.execute(
        down_at,
        {
            .t = std::int64_t{42},
        }
    );
```

A wrong parameter name SHOULD be a compile-time error.

A wrong statically known C++ type SHOULD be a compile-time error.

Rust SHALL still validate the actual parameter payload at execution.

Set parameters SHOULD accept borrowed spans:

```cpp
std::span<T const>
```

and SHALL NOT require an owning vector when the caller already owns storage.

---

# 22. Answers and result ownership

The bridge SHALL preserve Bumbledb's existing flat `Answers` representation rather than performing one FFI call per cell.

Rust `Answers` is already a reusable flat buffer containing fixed-width cells plus separate string and byte heaps. `clear()` retains capacity for reuse.

This is an excellent native-language boundary.

The C++ binding SHOULD expose an owning answers object backed by one opaque Rust allocation or one bridge-owned flattened buffer.

Conceptually:

```cpp
template<auto Query>
class Answers {
public:
    auto size() const -> std::size_t;

    auto operator[](
        std::size_t index
    ) const -> row_view;

    auto rows() const -> /* borrowed range */;
};
```

Variable-size answer values SHOULD be borrowed from the answer buffer:

```text
string answer  → string_view
bytes answer   → span<const byte>
```

Fixed-width values SHOULD be returned by value.

An answer-row view is valid only while its `Answers` owner remains alive and unchanged.

`Answers::get` panics on out-of-range access on the Rust side. The C++
accessors SHALL bounds-check on the C++ side of the boundary (contract
violation on `operator[]`; a checked `expected` accessor where recoverable
access is wanted) so no panic can originate from an index bug.

Execution takes the prepared query by `&mut` on the Rust side (reusable
scratch). The C++ `execute` SHALL therefore take `Prepared<Query>&` —
non-const — making exclusive use visible in the signature. This is the
enforcement of §20's no-concurrent-execution rule, not a style choice.

The first implementation SHOULD favor a whole-result crossing:

```text
execute once
    ↓
Rust produces Answers
    ↓
one bridge transfer/handle
    ↓
C++ iterates locally
```

rather than:

```text
execute
cell()
cell()
cell()
cell()
```

The Node bridge already crosses the `Answers` carrier as a unit and decodes it once at the language boundary.

---

# 23. Reusable answer buffers

Because Bumbledb's warm-path allocation model deliberately supports caller-owned reusable `Answers`, the C++ bridge SHOULD eventually expose that capability.

Desired advanced form:

```cpp
bdb::Answers<DownAt> answers;

auto status =
    snap.execute_into(
        prepared,
        params,
        answers
    );
```

Repeated execution reuses answer capacity.

A convenience form MAY allocate:

```cpp
auto answers =
    snap.execute(
        prepared,
        params
    );
```

but benchmark-sensitive code SHOULD be able to own and reuse the answer carrier.

The engine itself already distinguishes reusable output from a convenience fresh-buffer path.

---

# 24. Dynamic row marshalling

C++ reflection SHALL automatically lower ordinary relation row products to the engine's dynamic `Value` representation.

Given:

```cpp
ServiceRow{
    .id = id,
    .name = "search",
}
```

the C++ meta layer SHALL enumerate fields in declaration order and construct a borrowed/value bridge sequence equivalent to:

```text
[
    U64(id),
    String("search")
]
```

No serializer registration is permitted.

No manual `to_bumbledb()` implementation is permitted for ordinary supported row products.

No per-relation trait specialization is permitted.

The relation declaration itself and reflection are the source of truth.

For writes:

```cpp
tx.insert(
    Service,
    ServiceRow{
        ...
    }
)
```

SHOULD lower to the existing Rust:

```rust
tx.insert_dyn(relation_id, values)
```

The runtime dynamic shape check remains authoritative at the foreign boundary.

Bulk import (`bulk_load_dyn`) commits in atomic 4096-row chunks; prior chunks
stay committed on failure, and the importer owns dependency ordering — a
bidirectional (`==`) statement cluster must land within one chunk. The C++
bulk API SHALL surface both facts rather than burying them: its result
carries the committed count plus the typed error, mirroring Rust's
`BulkLoadError { committed, error }`.

---

# 25. Fresh IDs

The user SHOULD allocate fresh IDs by reflected field coordinate:

```cpp
auto id =
    tx.alloc(Service.id);
```

The C++ SDK SHALL resolve the field's relation and field IDs once from the admitted schema/manifest.

It SHALL use the existing dynamic fresh allocation mechanism rather than minting IDs itself.

No user-visible numeric relation/field IDs should normally appear in C++ application code.

The application deals in:

```cpp
Service.id
Program.grp
```

while the runtime bridge deals in numeric IDs.

---

# 26. Keyed reads

Statement values SHALL remain first-class.

This is a non-negotiable Bumbledb property.

The TypeScript cookbook explicitly stores the result of `key(...)` and later passes that exact law back to `get`.

C++:

```cpp
inline constexpr auto program_group_key =
    bdb::key(
        Program.grp
    );

inline constexpr auto KeyedRead =
    bdb::schema<"KeyedRead">(
        Grp,
        Program,

        bdb::contained(
            bdb::on(Program.grp),
            bdb::on(Grp.id)
        ),

        program_group_key
    );
```

Runtime:

```cpp
auto program =
    db.get(
        Program,
        program_group_key,
        {
            .grp = group,
        }
    );
```

There SHALL NOT be a generated nominal:

```cpp
ProgramByGroup
```

type whose only purpose is to stand in for the law.

The law itself is the selector.

Primary fresh-field reads MAY use the fresh field directly:

```cpp
db.get(
    Program,
    {
        .id = id,
    }
);
```

---

# 27. Errors

The C ABI SHALL NOT reduce engine errors to strings.

C++ SHALL receive structured recoverable errors.

Recommended boundary representation:

```c
typedef struct bdb_error bdb_error;

typedef enum {
    BDB_ERROR_SCHEMA,
    BDB_ERROR_SCHEMA_MISMATCH,
    BDB_ERROR_FORMAT_MISMATCH,
    BDB_ERROR_ALREADY_INITIALIZED,
    BDB_ERROR_NOT_INITIALIZED,
    BDB_ERROR_ENVIRONMENT_LOCKED,
    BDB_ERROR_STORE_KIND_MISMATCH,
    BDB_ERROR_DESCRIPTOR_MISSING,
    BDB_ERROR_READERS_FULL,
    BDB_ERROR_VALIDATION,
    BDB_ERROR_COMMIT_REJECTED,
    BDB_ERROR_COMMIT_SYNC,
    BDB_ERROR_GENERATION_MOVED,
    BDB_ERROR_FOREIGN_SNAPSHOT,
    BDB_ERROR_FOREIGN_PREPARED,
    BDB_ERROR_FACT_SHAPE,
    BDB_ERROR_CLOSED_RELATION_WRITE,
    BDB_ERROR_FRESH_EXHAUSTED,
    BDB_ERROR_BULK_LOAD,
    BDB_ERROR_PARAM,
    BDB_ERROR_MEASURE_OF_RAY,
    BDB_ERROR_CAPACITY_RAY_MEASURE,
    BDB_ERROR_FIXPOINT_BUDGET_EXCEEDED,
    BDB_ERROR_OVERFLOW,
    BDB_ERROR_RESULT_BYTES_OVERFLOW,
    BDB_ERROR_CORRUPTION,
    BDB_ERROR_IO,
    BDB_ERROR_LMDB,
    BDB_ERROR_PANIC
} bdb_error_kind;
```

The Rust `Error` has thirty variants and is deliberately not
`#[non_exhaustive]`. The bridge SHALL match it exhaustively so that a new
engine variant breaks the bridge compile — exactly the discipline the Node
bridge's `wire_tags!` tables enforce. `BDB_ERROR_PARAM` covers the bind-time
parameter family; `BDB_ERROR_PANIC` is bridge-synthesized (§30), never
engine-originated.

The C++ binding is the fourth spelling of these tag tables (Rust enum,
TypeScript union, `tags.json`, C header). The sync mechanism SHALL be
mechanical, not manual: the bridge's exhaustive `match` breaks compile on
engine drift, a pinned `cbindgen` regenerates the header, and a conformance
test pins the C enum values against a bridge-rendered golden, following the
`ts/test/wire-tags.test.ts` precedent.

The opaque `bdb_error` owns the Rust error payload.

Accessor functions SHALL expose structured payloads.

Formatting is a separate cold operation.

C++:

```cpp
class Error {
    foreign::error_handle handle_;

public:
    auto kind() const noexcept
        -> ErrorKind;

    auto message() const
        -> std::string;

    auto commit_rejection() const
        -> std::optional<CommitRejectionView>;

    // ...
};
```

The engine architecture explicitly distinguishes rich typed errors such as `CommitRejected`, `GenerationMoved`, `ForeignSnapshot`, `FactShape`, corruption and infrastructure failures; aborted write transactions leave the LMDB state untouched.

The C++ SDK should preserve that taxonomy.

---

# 28. Compile-time versus runtime error ownership

The frontend SHALL distinguish three failure layers.

A C++ structural authoring error SHOULD fail at compile time.

Examples:

```text
unsupported row field type
unknown reflected field
two fresh generators unified
query variable reused across incompatible law classes
closed handle from wrong vocabulary
invalid statically known projection structure
invalid result-record construction
```

A Rust schema/IR semantic validation error SHALL occur at:

```text
Db::create/open
prepare
```

and return `std::unexpected(bdb::Error)`.

An engine runtime/storage/transaction failure SHALL occur during execution or commit and likewise return typed runtime error data.

The C++ frontend MUST NOT claim a stronger theorem than it actually checked.

Rust validation remains a trust boundary.

---

# 29. Async and concurrency

The initial C++ SDK SHALL be synchronous.

This follows the existing engine's lexical snapshot/write design and keeps the foreign boundary simple.

The SDK SHALL NOT invent:

```text
future-returning DB operations
direct coroutines
background DB worker threads
detached operations
callback-asynchrony
```

If an application needs asynchronous composition, the synchronous Bumbledb operation SHALL be wrapped by the application's approved `std::execution` scheduler/runtime layer.

(Verified: the pinned GCC 16.1 libstdc++ ships no `std::execution`, and the
dialect's dormancy rule forbids substitute async mechanisms. Synchronous-only
is therefore not merely preferred — it is the only expressible design today,
and this SDK remains correct unchanged when senders arrive.)

The database SDK itself should remain:

```text
synchronous resource API
+
lexical snapshots
+
lexical writes
```

Prepared queries contain reusable mutable execution scratch and are intentionally not shareable concurrently in Rust.

The C++ SDK SHALL mirror this architecture rather than papering over it with locks or shared ownership.

---

# 30. Rust bridge safety discipline

The bridge crate lives on the unsafe boundary.

Its crate policy SHOULD match the existing Node bridge's style: unsafe code denied globally with explicit, locally justified carve-outs at the few unavoidable FFI/lifetime sites. The Node bridge currently uses that model.

Expected unsafe sites are narrowly limited to:

```text
opaque pointer validation/dereference
C callback invocation plumbing
prepared-query lifetime erasure
borrowed C slice/string construction
possibly answer-buffer raw views
```

Every unsafe site MUST include a concrete safety argument.

Panic policy is part of the bridge contract, not an afterthought. A Rust
panic unwinding across the C boundary into `-fno-exceptions` C++ is undefined
behavior. The bridge SHALL therefore:

```text
wrap every extern "C" entry point in std::panic::catch_unwind
    and map a caught panic to BDB_ERROR_PANIC
    (the caller treats the store as poisoned)

refuse re-entrant writes at the bridge layer with a typed error
    before the engine's assertion can fire (§17)

never reach Answers::get-style panicking accessors with unchecked
    indices (bounds are checked on the C++ side, §22)

keep unwinding inside Rust so the engine's own drop guards
    (e.g. the escaped-fresh-id burn on write failure) run as designed
```

No unsafe code SHALL be introduced into the core engine merely for C++ convenience unless a separate engine API improvement has independent justification.

The first implementation SHOULD attempt to add zero unsafe sites to existing core crates.

---

# 31. C++ foreign-zone containment

Only:

```text
cpp/foreign/
```

may see:

```cpp
extern "C"
bdb_db*
bdb_snapshot_ref*
bdb_tx_ref*
void*
C callback types
generated C header
```

`cpp/src/` MUST expose safe-ish value/resource wrappers.

For example, application code sees:

```cpp
bdb::Db
bdb::Snapshot
bdb::WriteTx
bdb::Prepared<Query>
bdb::Answers<Query>
bdb::Error
```

and never:

```cpp
bdb_db*
```

This is mandatory.

---

# 32. Build integration

The C++ build SHALL use the project's CMake + Ninja policy.

CMake SHALL build or locate the Rust bridge artifact and link it into the C++ SDK/runtime target.

A conceptual target graph:

```text
cargo build cpp/bridge
        ↓
libbumbledb_c.a
        ↓
bumbledb_foreign
        ↓
bumbledb_cpp
        ↓
application
```

Reflection code lives in `cpp/meta/`.

The GCC production graph compiles it.

The Clang lint graph excludes reflection-reachable modules until Clang can parse the required syntax, consistent with the existing dialect zoning strategy.

The C bridge header is a foreign generated/toolchain artifact and therefore does not violate the dialect's preprocessor ban.

No CMake header-unit workaround SHALL be introduced.

No PCH or unity build SHALL be introduced.

The dialect infrastructure is vendored from the cpp-starter template (§4).
Three pinned-toolchain facts from that template bind this SDK verbatim:

1. GCC 16.1 expansion statements (`template for`) trip `-Wshadow` on
   compiler-generated scoping whenever the expanded range has more than one
   element. This SDK is wall-to-wall expansion statements, so the reflection
   module set carries a scoped `-Wno-shadow` with the quirk pinned in a
   comment — never a per-line suppression.
2. `std::meta::define_aggregate` may only be evaluated from `consteval`
   blocks, which fixes the synthesis architecture to the
   class-template-scope injection pattern (§38).
3. The template's configure gate carries the CMake 4.2-series `import std`
   experimental UUID and links `-lstdc++exp` for the contracts violation
   handler; macOS builders inherit the template's `_rsize_t.h` fixinclude
   note (GCC's libstdc++ std module silently installs empty on macOS
   without it).

Because the SDK is reflection-centric, the Clang lint graph will cover little
more than `cpp/foreign/` wrappers and non-reflective tests until Clang parses
reflection. That is accepted: enforcement for the reflective core is GCC
diagnostics, the compile-fail suite and review — the dialect's enforcement
ladder — and the module graph SHALL NOT be contorted to enlarge lint
coverage.

CI SHALL land complete, not incrementally. The full matrix, pinned per the
starter template's CI discipline (exact container tags, checksummed CMake and
Ninja, SHA-pinned actions):

```text
gcc container (16.1):
    dev + release configure/build/ctest
        (unit, toolchain-conformance, cookbook runtime, lifetime tests)
    asan-ubsan job
    tsan job

lint job (LLVM 22 via apt.llvm.org):
    clang-tidy with the dialect custom checks over the Clang-readable graph

compile-fail job:
    the §34 expected-failure suite with pinned diagnostics

bridge job (pinned Rust nightly):
    cargo test for cpp/bridge (raw ABI tests, §35)
    cargo clippy under the workspace lint regime

parity gate:
    every ported cookbook recipe lowers through Rust to its golden in
    fixtures/cookbook-fingerprints.txt; a fingerprint mismatch fails CI
```

---

# 33. Cross-host parity is mandatory

The existing TypeScript cookbook is unusually valuable as the C++ SDK conformance corpus.

All 32 cookbook recipes SHOULD eventually exist under:

```text
cpp/tests/cookbook/
```

The C++ recipes SHALL represent the same theory, not a redesigned equivalent.

The initial parity definition is:

```text
same relation order
same field order
same structural field kinds
same fresh marks
same closed extension
same statements
same canonical descriptor
same fingerprint
same query/program IR meaning
same runtime answers
```

The C++ schema built for each cookbook recipe MUST lower through Rust to the same fingerprint already pinned cross-host.

The TypeScript cookbook itself states that its recipes are mechanically compiled/tested and that fingerprints are pinned cross-host to prove that the host surfaces describe the same theory.

C++ SHALL become another member of that parity set.

The golden set is one file with three readers and a host-neutral home: the
fingerprints file formerly in the TypeScript test tree
SHALL live at the repository root (`fixtures/cookbook-fingerprints.txt`),
and the TypeScript cookbook tests, the Rust `bumbledb-query` cookbook tests,
and the C++ cookbook tests SHALL all assert against that one path. No host
directory owns the goldens. Regeneration remains where the harness already
lives (the TypeScript runtime suite) until a better home is justified.

To serve the parity gate, the C ABI SHALL expose the admitted store's
fingerprint (the TypeScript bridge already reads it off the create outcome).

---

# 34. Compile-fail conformance suite

The SDK is not complete if only successful cookbook examples compile.

A dedicated compile-fail suite MUST prove that the C++ elaborator rejects invalid source programs.

Representative required failures include:

```text
relation contains unsupported C++ field type

relation contains raw pointer/reference field

two fresh field generators unified by a law

closed handle used against another closed relation

statement references a field outside the relation

query variable bound from one schema class reused against another

query parameter inferred inconsistently

result field produced from incompatible branch types

capacity weight refers to illegal nonlocal path

attempt to use a Snapshot outside its callback-facing API shape

attempt to copy Prepared/Db/Snapshot/WriteTx
```

Diagnostics SHOULD cite semantic coordinates such as:

```text
Outage.service
Service.id
Repo.id
```

rather than only template internals.

Reflection/template machinery is allowed to be ugly internally; compiler-facing diagnostics are part of the SDK product.

The harness itself is a Phase 0 deliverable: neither this repository nor the
starter template has one yet. It SHALL be a CTest-driven expected-failure
compiler harness — each compile-fail case is one translation unit that must
fail to compile AND must emit a pinned diagnostic substring — so a regression
that silently starts accepting an invalid program fails CI, and a diagnostic
that degrades into template noise fails review.

---

# 35. Runtime bridge tests

The bridge needs direct tests independent of reflection.

A small test-only C or minimal foreign harness SHOULD prove the raw ABI for:

```text
create/open/close
read callback
write callback
write abort
write_from
scan
insert/delete
fresh allocation
prepare
execute scalar params
execute set params
keyed get
bulk import
error destruction
prepared destruction
```

These tests answer:

> Is the foreign bridge correct?

The C++ cookbook tests answer:

> Is the reflective language correct?

Those should not be conflated.

---

# 36. Resource/lifetime tests

The following lifetime cases MUST be covered.

Destroying a database destroys all owned database state.

A prepared query keeps the engine state it borrows alive until the prepared object dies.

A snapshot cannot be used after `Db::read` returns.

A `WriteTx` cannot be used after `Db::write` returns.

An abandoned write commits nothing.

A callback-local failure aborts the delta.

A commit rejection returns the complete engine violation result.

Destroying an answers object invalidates its borrowed row/string/byte views.

Moving `Db`, `Prepared`, and `Answers` leaves the source in an inert valid state.

No public API requires a user-written close call.

RAII owns cleanup.

---

# 37. Performance constraints

The first bridge SHOULD optimize architecture before micro-optimizing calls.

The mandatory performance rules are:

```text
no FFI call per result cell

no callback worker thread for ordinary C++ snapshots/writes

no JSON schema/query marshalling

no string field lookup on the query execution hot path

no user-level shared ownership

no heap allocation merely to represent field coordinates

prepared queries remain reusable

answer capacity can eventually be reused

query/schema descriptors should live as compile-time/static data where possible
```

The bridge MUST NOT add speculative complex zero-copy machinery unless measurement justifies it.

The current engine already deliberately uses reusable `Answers` buffers and prepared-query scratch to control warm-path allocation.

The C++ SDK should avoid destroying those properties at the boundary.

---

# 38. Reflection spike required before implementation freeze

Before the entire SDK is built, implement a small isolated GCC 16 reflection spike proving the following pipeline:

```text
struct R {
    [[=bdb::fresh]] u64 id;
    string name;
};

        ↓ reflect

enumerate data members
read names
read types
read annotation

        ↓ synthesize

relation facade:
    R.id
    R.name

        ↓ transform

pattern/variable facade:
    vars.id
    vars.name

        ↓ result

synthesized answer product
```

This spike is the highest-risk C++-specific piece.

**Spike status: COMPLETE (2026-08-08, GCC 16.1.0).** All three mechanisms
verified on the pinned toolchain under
`-std=c++26 -freflection -fno-exceptions -fno-rtti`:

1. `[[=bdb::fresh]]` annotation syntax, `std::meta::annotations_of`, and
   annotation tag-type matching compile and evaluate.
2. `std::meta::define_aggregate` synthesizes aggregates whose member names
   come from another struct's reflected identifiers — but ONLY when evaluated
   from a `consteval` block. All synthesis therefore uses the
   class-template-scope injection pattern:

   ```cpp
   template<class Row>
   struct RelationTypes {
       struct Coords;
       consteval {
           std::meta::define_aggregate(^^Coords, coord_specs<Row>());
       }
   };
   ```

3. The full generic pipeline — `inline constexpr auto Service =
   make_facade<ServiceRow>();` yielding `Service.id.ordinal == 0` and
   `Service.id.fresh == true` — compiles and runs.

The public design target stands with no fallback needed. The original spike
question is answered affirmatively:

```cpp
Service.id
```

is produced cleanly by the pinned GCC implementation.

Do not respond by adding macros, manual registration, X-macro field lists, generated headers, CRTP registries or nominal field tags.

The dialect rule remains:

> Structural reflection is the source of truth.

---

# 39. First vertical slice

Do not begin by porting all 32 cookbook recipes.

The first slice SHALL demonstrate the entire architecture with one minimal theory equivalent to cookbook recipe 1.

It should support:

```cpp
struct ServiceRow {
    [[=bdb::fresh]]
    std::uint64_t id;
    std::string name;
};

struct OutageRow {
    std::uint64_t service;
    bdb::interval<std::int64_t> window;
};

inline constexpr auto Service =
    bdb::relation<"Service", ServiceRow>;

inline constexpr auto Outage =
    bdb::relation<"Outage", OutageRow>;

inline constexpr auto Uptime =
    bdb::schema<"Uptime">(
        Service,
        Outage,

        bdb::contained(
            bdb::on(Outage.service),
            bdb::on(Service.id)
        ),

        bdb::key(
            Outage.service,
            Outage.window
        )
    );
```

The slice SHALL prove:

```text
reflection produces relation fields
schema spec crosses C ABI
Rust admits theory
fingerprint equals existing cookbook recipe 1
Db creates/opens
write inserts Service + Outage
fresh alloc works
read callback works
one query prepares
query executes
answers decode
one deliberate pointwise-key violation returns structured rejection
```

Only after that end-to-end slice passes should the SDK expand outward.

---

# 40. Recommended implementation phases

The phases below are a dependency order, not calendar gates: this document
scopes the entire work so it can be executed as one comprehensive workflow.
CI is part of Phase 0's scaffold at full scope (§32) and each phase extends
what it gates; nothing about the matrix is deferred.

Phase 0 vendors the dialect infrastructure from the cpp-starter template into
`cpp/` (AGENTS.md, the clang-tidy custom-check configuration, format
configuration, presets, and the exact-tuple configure gate), stands up the
compile-fail CTest harness (§34), and lands the bridge panic-policy plumbing
decisions (§30). Nothing SDK-shaped exists before Phase 0 is green.

Phase A establishes the raw bridge and contains no C++ reflection beyond basic test scaffolding. Implement database ownership, lexical read/write callbacks, dynamic insert/delete/scan, schema-spec crossing, typed errors and bridge tests.

Phase B proves the C++26 relation reflector. Implement supported row type classification, fresh annotations, relation coordinate synthesis and row-to-Value marshalling.

Phase C implements schema statement values and consteval schema class elaboration. Recipe 1 must produce the existing fingerprint. Add compile-fail tests for generator collisions and invalid projections.

Phase D implements `query`, `vars`, `match`, `where`, `param`, `find`, IR lowering and preparation. Recipe 1 queries must match existing answers and IR validation.

Phase E implements prepared-query ownership, reusable answer carriers and typed synthesized query result views.

Phase F implements closed relations and payload-bearing closed vocabularies without reducing them to mere enums. Port recipes 2, 6, 7 and 8.

Phase G implements the remaining statement/query vocabulary, including `mirrors`, capacity, interval operations, aggregates, set parameters and recursion/programs.

Phase H ports all 32 cookbook recipes and makes fingerprint/runtime parity a required CI gate.

Phase I adds polished plan introspection, staleness and less-common operational surfaces after the core cookbook/runtime surface is complete.

---

# 41. Explicit non-goals

The first C++ SDK SHALL NOT:

```text
rewrite the Rust engine

reimplement LMDB/storage logic

reimplement the planner

JIT queries

generate arbitrary specialized C++ query executors

invent a second persisted schema wire format

replace Rust validation with C++ validation

introduce nominal ID wrapper types

introduce inheritance/interfaces

introduce application-visible raw pointers

introduce exceptions

introduce async database operations

introduce shared_ptr ownership

copy the Node worker-thread transaction architecture

expose numeric relation/field ids as the normal C++ API

use macros to register relation fields

use traits/registries where reflection can derive the fact
```

Static specialized C++ execution MAY be revisited someday only if a measured benchmark proves a material advantage over the existing Rust executor.

It is not part of this project.

---

# 42. Design invariants for the coding agent

The coding agent SHALL treat the following as hard architectural laws:

1. **Bumbledb semantics have one owner.** Rust/Lean remain authoritative; C++ is a host elaborator and binding.

2. **Declarations are the field source of truth.** A C++ row field is never restated in a schema field list.

3. **Statements type ordinary fields.** Ordinary entity references remain physical integers; semantic compatibility is derived from schema laws.

4. **Closed relations remain relations.** Host enums/handles are projections for ergonomics, not replacements for relational semantics.

5. **Laws are values.** A key statement can be stored and later used as the selector for a keyed read.

6. **Queries reuse the same semantic classes.** A variable's join compatibility comes from the admitted schema's law-computed classes, not just its physical C++ type.

7. **Compile-time richness should disappear.** Runtime rows should remain direct values and the Rust engine should receive ordinary descriptor/IR data.

8. **The foreign boundary is narrow and ugly.** Raw pointers and C ABI details stop in `cpp/foreign`.

9. **Snapshots and write transactions are lexical capabilities.** Do not make them independently owning handles merely because another binding did.

10. **Errors are data.** No exceptions, no boolean failure soup, no string-only engine failures.

11. **No shared ownership leaks upward.** Internal Rust `Arc` use for FFI lifetime ownership is allowed; C++ application semantics remain single-owner/value-first.

12. **Metaprogramming is ordinary programming.** Use reflection + consteval; do not create a second template-metaprogramming sublanguage.

13. **One concept gets one mechanism.** Do not keep a legacy/manual registration path beside the reflected path.

14. **Cross-host parity is the proof.** A C++ cookbook recipe is not done until it lands on the same Bumbledb theory/fingerprint and runtime semantics as the existing hosts.

---

# 43. Definition of done

The C++ SDK is considered complete for its first stable release when all 32 cookbook theories compile under the strict dialect, all schemas lower through the Rust `SchemaSpec` path, all cookbook fingerprints match their existing cross-host goldens, all cookbook query/program examples prepare through the existing Rust validator, runtime answer tests match the existing engine behavior, keyed reads preserve statement-as-selector semantics, closed relations remain fully relational and queryable, lexical read/write/write-from behavior maps directly to the Rust closure model, all application-facing errors are typed `expected` results, no application-facing raw pointer or preprocessor mechanism exists, and the dialect's enforcement ladder — compiler flags, build graph, clang-tidy over the lint graph, and the compile-fail suite — passes.

The desired final experience is:

```cpp
struct ServiceRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    std::string name;
};

struct OutageRow {
    std::uint64_t service;
    bdb::interval<std::int64_t> window;
};

inline constexpr auto Service =
    bdb::relation<"Service", ServiceRow>;

inline constexpr auto Outage =
    bdb::relation<"Outage", OutageRow>;

inline constexpr auto Uptime =
    bdb::schema<"Uptime">(
        Service,
        Outage,

        bdb::contained(
            bdb::on(Outage.service),
            bdb::on(Service.id)
        ),

        bdb::key(
            Outage.service,
            Outage.window
        )
    );

inline constexpr auto DownAt =
    bdb::query(Uptime)
        .rule([](auto r) consteval {
            auto vars =
                r.vars(Outage);

            return r
                .match(
                    Outage,
                    {
                        .service = vars.service,
                        .window = vars.window,
                    }
                )
                .where(
                    bdb::point_in(
                        r.param<"t">(),
                        vars.window
                    )
                )
                .find({
                    .service = vars.service,
                });
        });

auto main() -> int
{
    auto db =
        bdb::Db::create(
            "./uptime.db",
            Uptime
        );

    if (!db) {
        return report(db.error());
    }

    auto query =
        db->prepare<DownAt>();

    if (!query) {
        return report(query.error());
    }

    auto result =
        db->read([&](bdb::Snapshot& snap) {
            return snap.execute(
                *query,
                {
                    .t = std::int64_t{42},
                }
            );
        });

    return result
        .transform([](bdb::Answers<DownAt> const& rows) {
            for (auto const& row : rows.rows()) {
                std::println("service {} is down", row.service);
            }
            return 0;
        })
        .value_or(1);
}
```

The implementation underneath may contain reflection, synthesis, consteval algorithms, C ABI handles, Rust lifetime erasure and dynamic value marshalling.

The user-facing language should not.

The final SDK should feel like the existing structural Bumbledb theory expressed naturally in C++26, while execution remains the same Rust Bumbledb engine.
