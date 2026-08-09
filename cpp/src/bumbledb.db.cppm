// bumbledb.db — the runtime resource layer (TODO_CPP §15–§19, §24–§25).
//
// Zoning note (pinned): this module is dialect code under src/ but it is
// GCC-ONLY, because it imports the reflection-backed meta marshaller
// (bumbledb.meta.row) for tx.insert(Relation, Row). That is ACCEPTED
// (TODO_CPP §32: the reflective core's enforcement ladder is GCC
// diagnostics + compile-fail + review); the module itself contains no
// reflection syntax — everything reflective stays in meta/. The rest of
// the runtime layer (types, error, answers) remains Clang-visible.
//
// Two admission lanes: the SCHEMA lane (Db::create/open/ephemeral over a
// bdb::schema<> value, TODO_CPP §13) lowers the schema's flattened tables
// to the owned spec builder — declared statements only, newtype slots fed
// from the law-computed class map (lowering.md §2/§7) — and captures the
// manifest (relation ids + materialized statement ids for §26 keyed
// reads) from the same tables. The PRE-SCHEMA lane (raw
// bdb::foreign::bdb_schema_spec views) remains for spec-level tests.
// Coordinate → wire-id resolution works by NAME against the manifest:
// relation id = declaration index (the order IS the id mint, lowering.md
// §1.1), field id = the coordinate's reflected ordinal (ordinary
// relations: FieldId = declaration index, lowering.md §1.11).
//
// Failure taxonomy (§19, §27–§28): engine failure is std::unexpected
// (bdb::Error); domain abandonment is DATA on the success path
// (WriteOutcome::Abandoned); checked-value construction failure never
// reaches this module (bdb::TypeError, bumbledb.types).
export module bumbledb.db;

import std;
import bumbledb.error;
import bumbledb.answers;
import bumbledb.foreign;
import bumbledb.foreign.raii;
import bumbledb.foreign.program;
import bumbledb.meta.relation;
import bumbledb.meta.schema;
import bumbledb.meta.row;
import bumbledb.meta.query;

export namespace bdb {

/// The write callback's positive decision: commit the delta, carrying a
/// result value out of the callback.
template<class T>
struct Commit {
    using value_type = T;
    T value;
};

/// The write callback's negative decision AS DATA (§19): drop the delta —
/// LMDB never saw a fact — carrying the abandonment's own payload out.
/// Not an error and never the unexpected path.
template<class A>
struct Abandon {
    using value_type = A;
    A value;
};

/// What a write callback decides (§19).
template<class T, class A>
using WriteDecision = std::variant<Commit<T>, Abandon<A>>;

/// The valueless commit decision (`return bdb::commit();`).
constexpr auto commit() -> Commit<std::monostate> {
    return Commit<std::monostate>{std::monostate{}};
}

/// A value-carrying commit decision.
template<class T>
constexpr auto commit(T value) -> Commit<T> {
    return Commit<T>{std::move(value)};
}

/// The valueless abandon decision.
constexpr auto abandon() -> Abandon<std::monostate> {
    return Abandon<std::monostate>{std::monostate{}};
}

/// A value-carrying abandon decision (abandonment-as-data).
template<class A>
constexpr auto abandon(A value) -> Abandon<A> {
    return Abandon<A>{std::move(value)};
}

/// A committed write's outcome, carrying the Commit value.
template<class T>
struct Committed {
    T value;
};

/// An abandoned write's outcome, carrying the Abandon value.
template<class A>
struct Abandoned {
    A value;
};

/// What Db::write returns on the SUCCESS path (§19): the write either
/// committed or was abandoned by its own callback. Engine failure — commit
/// rejection included — is the expected's error path, never an alternative
/// here.
template<class T, class A>
using WriteOutcome = std::variant<Committed<T>, Abandoned<A>>;

/// The witnessed loop's honesty bound (the TS WITNESSED_ATTEMPT_CAP):
/// contention alone converges — each rerun reads a FRESHER snapshot; a
/// workload that moves the generation on EVERY one of this many
/// consecutive attempts is not converging and never will.
inline constexpr std::uint64_t witnessed_attempt_cap = 64;

/// The typed livelock refusal `Db::write_witnessed` answers past the cap:
/// every attempt found the generation moved, which is only sustainable
/// when the callback ITSELF (even indirectly) commits an interleaved
/// write each try. Host-policy pathology, not engine judgment — the
/// remedy is to move the interleaved write out of the callback. Carries
/// the final attempt's GenerationMoved error.
struct WitnessedLivelock {
    std::uint64_t attempts;
    Error last;
};

/// What a witnessed write can fail with: an engine failure (commit
/// rejection included), or the typed livelock refusal.
using WitnessedFailure = std::variant<Error, WitnessedLivelock>;

} // namespace bdb

namespace bdb::detail {

/// One materialized statement's structural identity: the keyed-read ABI
/// addresses statements by their fingerprint-pinned MATERIALIZED index
/// (fresh-implied keys first — relation order, then field order — then
/// the declared statements in written order; closed auto-keys arrive with
/// closed relations). Only keys resolve gets, but every statement holds
/// its slot so the ids stay aligned.
struct StatementRow {
    bool is_key;
    std::string relation;
    std::vector<std::string> projection;
};

/// The resolution table (see the module comment): relation names copied
/// at construction, declaration order = wire id; statement rows present
/// on the schema lane only (the pre-schema raw-spec lane resolves no
/// keyed reads).
struct Manifest {
    std::vector<std::string> relation_names;
    std::vector<StatementRow> statement_rows;

    [[nodiscard]] auto resolve(std::string_view relation) const
        -> std::optional<std::uint32_t> {
        for (auto const& [index, name] :
            std::views::enumerate(relation_names)) {
            if (name == relation) {
                return static_cast<std::uint32_t>(index);
            }
        }
        return std::nullopt;
    }

    /// The key statement with exactly this structural identity — the §26
    /// law-value selector, resolved by content, never by a nominal type.
    [[nodiscard]] auto resolve_key(std::string_view relation,
        std::span<std::string_view const> projection) const
        -> std::optional<std::uint16_t> {
        for (auto const& [index, row] :
            std::views::enumerate(statement_rows)) {
            if (!row.is_key || row.relation != relation
                || row.projection.size() != projection.size()) {
                continue;
            }
            if (std::ranges::equal(row.projection, projection)) {
                return static_cast<std::uint16_t>(index);
            }
        }
        return std::nullopt;
    }

    /// The relation's PRIMARY key: its first key statement in
    /// materialized order (a fresh-bearing relation's fresh field —
    /// lowering.md §5.3).
    [[nodiscard]] auto resolve_primary(std::string_view relation) const
        -> std::optional<std::uint16_t> {
        for (auto const& [index, row] :
            std::views::enumerate(statement_rows)) {
            if (row.is_key && row.relation == relation) {
                return static_cast<std::uint16_t>(index);
            }
        }
        return std::nullopt;
    }
};

/// Resolves or dies: a coordinate/facade naming a relation outside the
/// admitted spec is an impossible programmer state (the facade and the
/// spec are both compile-time artifacts of the same declaration set), not
/// a recoverable input.
auto resolved_relation(Manifest const& manifest, std::string_view relation)
    -> std::uint32_t {
    auto const id = manifest.resolve(relation);
    contract_assert(id.has_value());
    return *id;
}

/// The facade's relation name, read off its first coordinate (every
/// coordinate of one facade carries the same relation name). C++26
/// structured-binding packs (P1061) — no reflection syntax, which is what
/// keeps this module out of meta/.
template<class Facade>
constexpr auto facade_relation_name(Facade const& facade)
    -> std::string_view {
    auto const& [...coords] = facade;
    static_assert(sizeof...(coords) > 0);
    return [](auto const& first, auto const&...) {
        return first.relation();
    }(coords...);
}

auto lift(foreign::error_handle handle) -> Error {
    return Error{std::move(handle)};
}

/// A keyed read's outcome: a hit is one owned row; a miss is genuine
/// absence (the ABI wrote no row set).
auto lift_row(foreign::row_set_handle handle) -> std::optional<RowSet> {
    auto rows = RowSet{std::move(handle)};
    if (rows.len() == 0) {
        return std::nullopt;
    }
    return rows;
}

// --- the schema wire lane (lowering.md §2): flattened schema tables
// lowered to the foreign owned spec builder --------------------------------

auto wire_type_of(field_data const& field) -> foreign::bdb_value_type {
    switch (field.kind) {
    case value_kind::boolean:
        return foreign::scalar_type(
            foreign::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_BOOL);
    case value_kind::u64:
        return foreign::scalar_type(
            foreign::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_U64);
    case value_kind::i64:
        return foreign::scalar_type(
            foreign::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_I64);
    case value_kind::string:
        return foreign::scalar_type(
            foreign::bdb_value_type_kind::BDB_VALUE_TYPE_KIND_STRING);
    case value_kind::fixed_bytes:
        return foreign::fixed_bytes_type(field.fixed_len);
    case value_kind::interval_u64:
        return field.width == 0
            ? foreign::interval_type(
                  foreign::bdb_interval_element::BDB_INTERVAL_ELEMENT_U64)
            : foreign::fixed_interval_type(
                  foreign::bdb_interval_element::BDB_INTERVAL_ELEMENT_U64,
                  field.width);
    case value_kind::interval_i64:
        break;
    }
    return field.width == 0
        ? foreign::interval_type(
              foreign::bdb_interval_element::BDB_INTERVAL_ELEMENT_I64)
        : foreign::fixed_interval_type(
              foreign::bdb_interval_element::BDB_INTERVAL_ELEMENT_I64,
              field.width);
}

/// The coordinate's law-computed class name, rendered "Relation.field"
/// for the newtype slot (lowering.md §1.10/§7.7); nullopt on bare.
template<class Classes>
auto newtype_of(Classes const& classes, name_text relation,
    name_text field) -> std::optional<std::string> {
    for (auto const& entry : classes) {
        if (entry.coordinate.relation == relation
            && entry.coordinate.field == field) {
            if (!entry.classed) {
                return std::nullopt;
            }
            return std::string{entry.class_name.relation.view()} + "."
                + std::string{entry.class_name.field.view()};
        }
    }
    return std::nullopt;
}

/// One schema-lane σ/axiom literal, owned (handles cross BY NAME —
/// lowering.md §7.8; values tagged).
auto owned_literal_of(selection_literal const& literal)
    -> foreign::owned_literal {
    auto out = foreign::owned_literal{};
    if (literal.is_handle) {
        out.is_handle = true;
        out.handle = std::string{literal.handle.view()};
        return out;
    }
    switch (literal.kind) {
    case value_kind::boolean:
        out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_BOOL;
        out.boolean = literal.boolean;
        return out;
    case value_kind::u64:
        out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_U64;
        out.u64 = literal.u64;
        return out;
    case value_kind::i64:
        out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_I64;
        out.i64 = literal.i64;
        return out;
    case value_kind::string:
    case value_kind::fixed_bytes:
    case value_kind::interval_u64:
    case value_kind::interval_i64:
        break;
    }
    out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_STRING;
    out.text = std::string{literal.text.view()};
    return out;
}

auto owned_axiom_of(axiom_literal const& literal)
    -> foreign::owned_literal {
    auto out = foreign::owned_literal{};
    switch (literal.kind) {
    case value_kind::boolean:
        out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_BOOL;
        out.boolean = literal.boolean;
        return out;
    case value_kind::u64:
        out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_U64;
        out.u64 = literal.u64;
        return out;
    case value_kind::i64:
        out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_I64;
        out.i64 = literal.i64;
        return out;
    case value_kind::string:
    case value_kind::fixed_bytes:
    case value_kind::interval_u64:
    case value_kind::interval_i64:
        break;
    }
    out.kind = foreign::bdb_value_kind::BDB_VALUE_KIND_STRING;
    out.text = std::string{literal.text.view()};
    return out;
}

template<Theory S>
auto owned_relations_of(S const& theory)
    -> std::vector<foreign::owned_relation> {
    auto relations = std::vector<foreign::owned_relation>{};
    relations.reserve(theory.relation_table.size());
    for (auto const& relation : theory.relation_table) {
        // A CLOSED relation's declared FieldSpecs are its intrinsic
        // payload columns ONLY — the synthetic id (sealed index 0 of the
        // flattened roster) is materialized by engine validation, never
        // spelled in the spec (lowering.md §7.3).
        auto const first_field =
            relation.closed ? std::size_t{1} : std::size_t{0};
        auto fields = std::vector<foreign::owned_field>{};
        fields.reserve(relation.field_count - first_field);
        for (auto index = first_field; index != relation.field_count;
            ++index) {
            auto const& field = relation.fields[index];
            fields.push_back(foreign::owned_field{
                .name = std::string{field.name.view()},
                .value_type = wire_type_of(field),
                .newtype = newtype_of(
                    theory.classes, relation.name, field.name),
                .fresh = field.fresh,
            });
        }
        auto closed = std::optional<foreign::owned_closed>{};
        if (relation.closed) {
            auto const& data = relation.closed_data;
            auto rows = std::vector<foreign::owned_closed_row>{};
            rows.reserve(data.handle_count);
            for (auto handle = std::size_t{0};
                handle != data.handle_count; ++handle) {
                auto values = std::vector<foreign::owned_literal>{};
                values.reserve(data.column_count);
                for (auto column = std::size_t{0};
                    column != data.column_count; ++column) {
                    values.push_back(owned_axiom_of(
                        data.axioms[handle * max_closed_columns
                            + column]));
                }
                rows.push_back(foreign::owned_closed_row{
                    .handle = std::string{data.handles[handle].view()},
                    .values = std::move(values),
                });
            }
            // ClosedSpec.newtype is ALWAYS the id's generator class
            // "<Name>.id" (lowering.md §7.7).
            closed = foreign::owned_closed{
                .newtype = std::string{relation.name.view()} + ".id",
                .rows = std::move(rows),
            };
        }
        relations.push_back(foreign::owned_relation{
            .name = std::string{relation.name.view()},
            .fields = std::move(fields),
            .closed = std::move(closed),
        });
    }
    return relations;
}

auto owned_side_of(side_data const& side) -> foreign::owned_side {
    auto projection = std::vector<std::string>{};
    projection.reserve(side.width);
    for (auto index = std::size_t{0}; index != side.width; ++index) {
        projection.emplace_back(side.fields[index].view());
    }
    auto selection = std::vector<foreign::owned_selection>{};
    selection.reserve(side.selection_count);
    for (auto binding = std::size_t{0}; binding != side.selection_count;
        ++binding) {
        auto const& data = side.selections[binding];
        auto literals = std::vector<foreign::owned_literal>{};
        literals.reserve(data.literal_count);
        for (auto literal = std::size_t{0};
            literal != data.literal_count; ++literal) {
            literals.push_back(owned_literal_of(data.literals[literal]));
        }
        selection.push_back(foreign::owned_selection{
            .field = std::string{data.field.view()},
            .literals = std::move(literals),
        });
    }
    return foreign::owned_side{
        .relation = std::string{side.relation.view()},
        .projection = std::move(projection),
        .selection = std::move(selection),
    };
}

auto owned_bound_of(bound_data const& bound) -> foreign::owned_bound {
    auto const kind = [&] {
        switch (bound.form) {
        case bound_form::lit:
            return foreign::bdb_bound_kind::BDB_BOUND_KIND_LIT;
        case bound_form::field:
            return foreign::bdb_bound_kind::BDB_BOUND_KIND_FIELD;
        case bound_form::duration_field:
            break;
        }
        return foreign::bdb_bound_kind::BDB_BOUND_KIND_DURATION_FIELD;
    }();
    return foreign::owned_bound{
        .kind = kind,
        .lit = bound.lit,
        .field = std::string{bound.field.view()},
    };
}

template<Theory S>
auto owned_statements_of(S const& theory)
    -> std::vector<foreign::owned_statement> {
    auto statements = std::vector<foreign::owned_statement>{};
    statements.reserve(theory.statements.size());
    for (auto const& statement : theory.statements) {
        switch (statement.form) {
        case statement_form::key: {
            auto side = owned_side_of(statement.source);
            statements.push_back(foreign::owned_fd{
                .relation = std::move(side.relation),
                .projection = std::move(side.projection),
            });
            break;
        }
        case statement_form::containment:
            statements.push_back(foreign::owned_containment{
                .source = owned_side_of(statement.source),
                .target = owned_side_of(statement.target),
                .bidirectional = statement.bidirectional,
            });
            break;
        case statement_form::capacity: {
            auto const weight_kind = [&] {
                switch (statement.weight) {
                case weight_form::unit:
                    return foreign::bdb_weight_kind::BDB_WEIGHT_KIND_UNIT;
                case weight_form::field:
                    return foreign::bdb_weight_kind::
                        BDB_WEIGHT_KIND_FIELD;
                case weight_form::duration_field:
                    break;
                }
                return foreign::bdb_weight_kind::
                    BDB_WEIGHT_KIND_DURATION_FIELD;
            }();
            auto const window_kind = [&] {
                switch (statement.window.form) {
                case window_form::exact:
                    return foreign::bdb_capacity_window_kind::
                        BDB_CAPACITY_WINDOW_KIND_EXACT;
                case window_form::range:
                    return foreign::bdb_capacity_window_kind::
                        BDB_CAPACITY_WINDOW_KIND_RANGE;
                case window_form::floor:
                    break;
                }
                return foreign::bdb_capacity_window_kind::
                    BDB_CAPACITY_WINDOW_KIND_FLOOR;
            }();
            statements.push_back(foreign::owned_capacity{
                .target = owned_side_of(statement.target),
                .weight = foreign::owned_weight{
                    .kind = weight_kind,
                    .field = std::string{statement.weight_field.view()},
                },
                .window = foreign::owned_capacity_window{
                    .kind = window_kind,
                    .lo = owned_bound_of(statement.window.lo),
                    .hi = owned_bound_of(statement.window.hi),
                },
                .source = owned_side_of(statement.source),
            });
            break;
        }
        }
    }
    return statements;
}

/// The schema lane's resolution table: relation names in declaration
/// order plus the MATERIALIZED statement identities (fresh-implied keys
/// first, declared statements after — the keyed-read id space).
template<Theory S>
auto manifest_of(S const& theory) -> Manifest {
    auto manifest = Manifest{};
    manifest.relation_names.reserve(theory.relation_table.size());
    for (auto const& relation : theory.relation_table) {
        manifest.relation_names.emplace_back(relation.name.view());
    }
    for (auto const& relation : theory.relation_table) {
        for (auto index = std::size_t{0}; index != relation.field_count;
            ++index) {
            auto const& field = relation.fields[index];
            if (!field.fresh) {
                continue;
            }
            manifest.statement_rows.push_back(StatementRow{
                .is_key = true,
                .relation = std::string{relation.name.view()},
                .projection = {std::string{field.name.view()}},
            });
        }
    }
    // Closed auto-keys follow the fresh-implied keys in MATERIALIZED
    // order (one `R(id) -> R` per closed relation, declaration order —
    // lowering.md §2), keeping keyed-read statement ids aligned.
    for (auto const& relation : theory.relation_table) {
        if (!relation.closed) {
            continue;
        }
        manifest.statement_rows.push_back(StatementRow{
            .is_key = true,
            .relation = std::string{relation.name.view()},
            .projection = {std::string{"id"}},
        });
    }
    for (auto const& statement : theory.statements) {
        auto row = StatementRow{
            .is_key = statement.form == statement_form::key,
            .relation = std::string{statement.source.relation.view()},
            .projection = {},
        };
        if (row.is_key) {
            row.projection.reserve(statement.source.width);
            for (auto index = std::size_t{0};
                index != statement.source.width; ++index) {
                row.projection.emplace_back(
                    statement.source.fields[index].view());
            }
        }
        manifest.statement_rows.push_back(std::move(row));
    }
    return manifest;
}

/// Resolves a stored key law to its materialized statement id, or dies:
/// passing a law from OUTSIDE the admitted schema (or keyed-reading a
/// pre-schema-lane store) is an impossible programmer state — the law and
/// the manifest are both artifacts of the same declaration set.
template<class First, class... Rest>
auto resolved_key(Manifest const& manifest,
    key_law<First, Rest...> const&) -> std::uint16_t {
    using Law = key_law<First, Rest...>;
    auto names = std::array<std::string_view, Law::width>{};
    for (auto index = std::size_t{0}; index != Law::width; ++index) {
        names[index] = Law::projection[index].view();
    }
    auto const id = manifest.resolve_key(Law::relation_name.view(), names);
    contract_assert(id.has_value());
    return *id;
}

/// Resolves a relation's primary key statement, or dies (as above: a
/// fresh-bearing facade of the admitted schema always has one).
auto resolved_primary(Manifest const& manifest, std::string_view relation)
    -> std::uint16_t {
    auto const id = manifest.resolve_primary(relation);
    contract_assert(id.has_value());
    return *id;
}

/// The §26 facade/law agreement diagnostic.
template<class Facade, class Law>
consteval auto keyed_get_mismatch() -> std::string {
    return std::string{"bumbledb get(): the key law constrains relation \""}
        + std::string{Law::relation_name.view()}
        + "\" but the facade names relation \""
        + std::string{facade_relation_name(Facade{})} + "\"";
}

} // namespace bdb::detail

export namespace bdb {

/// A reusable prepared query (TODO_CPP §20): move-only RAII over the
/// bridge's prepared handle. The engine validated, normalized, and
/// planned ONCE at `Db::prepare<Query>()`; the handle is reusable across
/// snapshots of the same database. Concurrent execution through one
/// prepared object is outside the dialect's permitted model — execution
/// takes it non-const (§22).
template<auto Query>
class Prepared {
    foreign::prepared_handle handle_;

    explicit Prepared(foreign::prepared_handle handle)
        : handle_{std::move(handle)} {}

    friend class Db;

public:
    Prepared(Prepared const&) = delete;
    auto operator=(Prepared const&) -> Prepared& = delete;
    Prepared(Prepared&&) noexcept = default;
    auto operator=(Prepared&&) noexcept -> Prepared& = default;
    ~Prepared() = default;

    /// Whether this handle still owns a prepared query (false after
    /// move-out — the §36 inert-source witness).
    [[nodiscard]] auto alive() const -> bool {
        return handle_.alive();
    }

    /// The bridge lane (Snapshot::execute drives it).
    [[nodiscard]] auto native() -> foreign::prepared_handle& {
        return handle_;
    }
};

/// The typed answers carrier of one query (TODO_CPP §12, §22–§23):
/// `bdb::Answers<DownAt>` decodes rows as the synthesized row product of
/// DownAt's `.find` head — named members, fixed-width by value,
/// string_view/span borrowed from the carrier.
template<auto Query>
using Answers = RowAnswers<row_of<Query>>;

/// A lexical borrowed read capability (§16): alive exactly for the
/// Db::read callback. Non-copyable, non-movable, constructible only by
/// Db's trampoline; it never owns and never outlives the callback frame.
class Snapshot {
    foreign::bdb_snapshot_ref const& raw_;
    detail::Manifest const& manifest_;

    Snapshot(foreign::bdb_snapshot_ref const& raw,
        detail::Manifest const& manifest)
        : raw_{raw}, manifest_{manifest} {}

    friend class Db;

public:
    Snapshot(Snapshot const&) = delete;
    auto operator=(Snapshot const&) -> Snapshot& = delete;
    ~Snapshot() = default;

    /// Committed-state membership of one row (marshalled by reflection in
    /// declaration order, §24).
    template<class Facade, class Row>
    [[nodiscard]] auto contains(Facade const& relation, Row const& row) const
        -> std::expected<bool, Error> {
        auto const cells = marshal_row(row);
        return foreign::snapshot_contains(raw_,
            detail::resolved_relation(
                manifest_, detail::facade_relation_name(relation)),
            cells)
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Full-relation export in row_id order: ONE owned crossing, iterated
    /// host-side (§37) — cells decode to bdb::Value; typed row decode
    /// arrives with the schema phase.
    template<class Facade>
    [[nodiscard]] auto scan(Facade const& relation) const
        -> std::expected<RowSet, Error> {
        return foreign::snapshot_scan(raw_,
            detail::resolved_relation(
                manifest_, detail::facade_relation_name(relation)))
            .transform([](foreign::row_set_handle handle) {
                return RowSet{std::move(handle)};
            })
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Committed-state keyed point read (§26): the stored key law value
    /// IS the selector — resolved against the schema's materialized
    /// statements by structural identity, never through a generated
    /// nominal type. Key values arrive as the law's pattern product,
    /// members in projection order. A miss is genuine absence.
    template<class Facade, class First, class... Rest>
    [[nodiscard]] auto get(Facade const& relation,
        key_law<First, Rest...> const& law,
        typename key_law<First, Rest...>::pattern const& key) const
        -> std::expected<std::optional<RowSet>, Error> {
        static_assert(detail::facade_relation_name(Facade{})
                == key_law<First, Rest...>::relation_name.view(),
            detail::keyed_get_mismatch<Facade,
                key_law<First, Rest...>>());
        auto const cells = marshal_row(key);
        return foreign::snapshot_get(raw_,
            detail::resolved_relation(
                manifest_, detail::facade_relation_name(relation)),
            detail::resolved_key(manifest_, law), cells)
            .transform(detail::lift_row)
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// The fresh-field primary read (§26): `snap.get(Service, {.id = id})`
    /// reads through the relation's PRIMARY key — the first materialized
    /// key, i.e. the fresh field's implied key.
    template<class Facade>
        requires (fresh_field_count<Facade>() >= 1)
    [[nodiscard]] auto get(Facade const& relation,
        fresh_pattern_of<Facade> const& key) const
        -> std::expected<std::optional<RowSet>, Error> {
        auto const cells = marshal_row(key);
        return foreign::snapshot_get(raw_,
            detail::resolved_relation(
                manifest_, detail::facade_relation_name(relation)),
            detail::resolved_primary(
                manifest_, detail::facade_relation_name(relation)),
            cells)
            .transform(detail::lift_row)
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Executes a prepared query into the caller's reusable carrier
    /// (§23's zero-alloc lane): the carrier is cleared first, capacity
    /// retained. Params arrive as the query's synthesized product —
    /// `{.t = std::int64_t{42}}` — so a wrong name or type is a compile
    /// error (§21); the engine still validates the payload at bind.
    template<auto Query>
    [[nodiscard]] auto execute_into(Prepared<Query>& prepared,
        params_of<Query> const& params, Answers<Query>& answers) const
        -> std::expected<void, Error> {
        // The scratch owns any runtime ∈-set cells for exactly this call
        // (the bridge copies before returning).
        auto scratch = foreign::param_scratch{};
        auto const wire = foreign::wire_params_for<Query>(params, scratch);
        return prepared.native()
            .execute(raw_, wire, answers.native().native())
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// The convenience execute (§22's whole-result crossing): one bridge
    /// transfer, iterated locally through the typed rows() range.
    template<auto Query>
    [[nodiscard]] auto execute(Prepared<Query>& prepared,
        params_of<Query> const& params) const
        -> std::expected<Answers<Query>, Error> {
        auto answers = Answers<Query>{};
        return execute_into<Query>(prepared, params, answers)
            .transform([&answers] { return std::move(answers); });
    }
};

/// A lexical borrowed write capability (§17): alive exactly for the
/// Db::write / Db::write_from callback. Non-copyable, non-movable,
/// constructible only by Db's trampoline. Nothing is judged until commit;
/// the callback's decision (§19) is the commit/abandon switch.
class WriteTx {
    foreign::bdb_tx_ref& raw_;
    detail::Manifest const& manifest_;

    WriteTx(foreign::bdb_tx_ref& raw, detail::Manifest const& manifest)
        : raw_{raw}, manifest_{manifest} {}

    friend class Db;

    [[nodiscard]] auto relation_id(std::string_view relation) const
        -> std::uint32_t {
        return detail::resolved_relation(manifest_, relation);
    }

public:
    WriteTx(WriteTx const&) = delete;
    auto operator=(WriteTx const&) -> WriteTx& = delete;
    ~WriteTx() = default;

    /// Records an insert into the delta (reflection-marshalled, §24);
    /// true = the final state changed. Shape violations are the engine's
    /// typed FactShape error. This phase does not type-check Row against
    /// Facade — that theorem belongs to the schema phase (§28).
    template<class Facade, class Row>
    [[nodiscard]] auto insert(Facade const& relation, Row const& row)
        -> std::expected<bool, Error> {
        auto const cells = marshal_row(row);
        return foreign::tx_insert(raw_,
            relation_id(detail::facade_relation_name(relation)), cells)
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Records a delete into the delta; true = the final state changed.
    template<class Facade, class Row>
    [[nodiscard]] auto remove(Facade const& relation, Row const& row)
        -> std::expected<bool, Error> {
        auto const cells = marshal_row(row);
        return foreign::tx_remove(raw_,
            relation_id(detail::facade_relation_name(relation)), cells)
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Final-state membership (base + pending delta — what the commit
    /// judgment judges; check-then-act is race-free under the single
    /// writer).
    template<class Facade, class Row>
    [[nodiscard]] auto contains(Facade const& relation, Row const& row) const
        -> std::expected<bool, Error> {
        auto const cells = marshal_row(row);
        return foreign::tx_contains(raw_,
            relation_id(detail::facade_relation_name(relation)), cells)
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Final-state keyed point read (§26, the WriteTx twin): the stored
    /// key law value is the selector; reads base + pending delta.
    template<class Facade, class First, class... Rest>
    [[nodiscard]] auto get(Facade const& relation,
        key_law<First, Rest...> const& law,
        typename key_law<First, Rest...>::pattern const& key) const
        -> std::expected<std::optional<RowSet>, Error> {
        static_assert(detail::facade_relation_name(Facade{})
                == key_law<First, Rest...>::relation_name.view(),
            detail::keyed_get_mismatch<Facade,
                key_law<First, Rest...>>());
        auto const cells = marshal_row(key);
        return foreign::tx_get(raw_,
            relation_id(detail::facade_relation_name(relation)),
            detail::resolved_key(manifest_, law), cells)
            .transform(detail::lift_row)
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// The fresh-field primary read against the final state (§26).
    template<class Facade>
        requires (fresh_field_count<Facade>() >= 1)
    [[nodiscard]] auto get(Facade const& relation,
        fresh_pattern_of<Facade> const& key) const
        -> std::expected<std::optional<RowSet>, Error> {
        auto const cells = marshal_row(key);
        return foreign::tx_get(raw_,
            relation_id(detail::facade_relation_name(relation)),
            detail::resolved_primary(
                manifest_, detail::facade_relation_name(relation)),
            cells)
            .transform(detail::lift_row)
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Mints the next fresh id for the coordinate's field (§25):
    /// `tx.alloc(Service.id)`. The coordinate carries relation name and
    /// ordinal in its type; resolution is the pre-schema name lane (module
    /// comment). Fresh fields are u64 by construction, so only u64
    /// coordinates allocate.
    template<class Field>
        requires std::same_as<typename Field::value_type, std::uint64_t>
    [[nodiscard]] auto alloc(Field const& field)
        -> std::expected<std::uint64_t, Error> {
        return foreign::tx_alloc(raw_, relation_id(field.relation()),
            static_cast<std::uint16_t>(Field::ordinal))
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }
};

} // namespace bdb

namespace bdb::detail {

// Pattern-match of a write body's required result shape:
// std::expected<WriteDecision<T, A>, Error>. The primary stays undefined
// so a mis-shaped body fails the WriteBody concept, not an instantiation
// deep inside Db::write.
template<class BodyResult>
struct WriteShapeOf;

template<class T, class A>
struct WriteShapeOf<std::expected<std::variant<Commit<T>, Abandon<A>>, Error>> {
    using CommitCase = Commit<T>;
    using AbandonCase = Abandon<A>;
    using Outcome = WriteOutcome<T, A>;
    using Result = std::expected<Outcome, Error>;
};

template<class Body>
using WriteShape = WriteShapeOf<std::invoke_result_t<Body&, WriteTx&>>;

// The witnessed twin: the body takes the WITNESSING snapshot and the tx
// (premise reads on snap, the delta on tx — TODO_CPP §18).
template<class BodyResult>
struct WitnessedShapeOf;

template<class T, class A>
struct WitnessedShapeOf<
    std::expected<std::variant<Commit<T>, Abandon<A>>, Error>> {
    using Outcome = WriteOutcome<T, A>;
    using Result = std::expected<Outcome, WitnessedFailure>;
};

template<class Body>
using WitnessedShape =
    WitnessedShapeOf<std::invoke_result_t<Body&, Snapshot&, WriteTx&>>;

template<class Result>
inline constexpr bool is_error_expected = false;

template<class T>
inline constexpr bool is_error_expected<std::expected<T, Error>> = true;

} // namespace bdb::detail

export namespace bdb {

/// A read body: Snapshot& -> std::expected<R, Error>.
template<class Body>
concept ReadBody = std::invocable<Body&, Snapshot&>
    && detail::is_error_expected<std::invoke_result_t<Body&, Snapshot&>>;

/// A write body: WriteTx& -> std::expected<WriteDecision<T, A>, Error>.
template<class Body>
concept WriteBody = std::invocable<Body&, WriteTx&>
    && requires { typename detail::WriteShape<Body>::Result; };

/// A witnessed-write body: (Snapshot&, WriteTx&) ->
/// std::expected<WriteDecision<T, A>, Error> — premise reads on the
/// snapshot, the delta on the tx (TODO_CPP §18).
template<class Body>
concept WitnessedBody = std::invocable<Body&, Snapshot&, WriteTx&>
    && requires { typename detail::WitnessedShape<Body>::Result; };

/// The owning database capability (§15): move-only RAII; no shared
/// ownership exists at this API. The moved-from Db is inert
/// (alive() == false); RAII owns cleanup — there is no close().
class Db {
    foreign::db_handle handle_;
    detail::Manifest manifest_;

    Db(foreign::db_handle handle, detail::Manifest manifest)
        : handle_{std::move(handle)}, manifest_{std::move(manifest)} {}

    static auto admit(
        std::expected<foreign::db_handle, foreign::error_handle> opened,
        foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
        return std::move(opened)
            .transform([&spec](foreign::db_handle handle) {
                return Db{std::move(handle),
                    detail::Manifest{
                        .relation_names = foreign::relation_names_of(spec),
                        .statement_rows = {},
                    }};
            })
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    // The §19 algebra, shared by write and write_from. The optional slot
    // smuggles the C++ body's full result through the C trampoline;
    // OK/ABORT is derived from it — Commit is the ONLY OK — so
    // user-abandon and user-error both abort the delta but stay
    // distinguishable on the way out.
    template<WriteBody Body, class Runner>
    auto write_through(Body& body, Runner runner) ->
        typename detail::WriteShape<Body>::Result {
        using Shape = detail::WriteShape<Body>;
        using Result = typename Shape::Result;
        using BodyResult = std::invoke_result_t<Body&, WriteTx&>;

        auto slot = std::optional<BodyResult>{};
        auto shim = [&](foreign::bdb_tx_ref& transaction)
            -> foreign::bdb_callback_control {
            auto tx = WriteTx{transaction, manifest_};
            slot.emplace(body(tx));
            auto const wants_commit = slot->has_value()
                && std::holds_alternative<typename Shape::CommitCase>(
                    **slot);
            return wants_commit
                ? foreign::bdb_callback_control::BDB_CALLBACK_CONTROL_OK
                : foreign::bdb_callback_control::BDB_CALLBACK_CONTROL_ABORT;
        };
        auto outcome = runner(shim);
        if (!outcome.has_value()) {
            // Engine failure — commit rejection included (§19's
            // unexpected path).
            return Result{
                std::unexpect, detail::lift(std::move(outcome).error())};
        }
        contract_assert(slot.has_value());
        if (*outcome == foreign::callback_done::completed) {
            contract_assert(slot->has_value());
            return Result{typename Shape::Outcome{Committed{std::move(
                std::get<typename Shape::CommitCase>(**slot).value)}}};
        }
        if (!slot->has_value()) {
            // The body's own typed failure aborted the delta (§36:
            // callback-local failure commits nothing).
            return Result{std::unexpect, std::move(*slot).error()};
        }
        // Abandonment-as-data: the delta dropped, the payload survives.
        return Result{typename Shape::Outcome{Abandoned{std::move(
            std::get<typename Shape::AbandonCase>(**slot).value)}}};
    }

    // The schema lane's admission: the spec views live exactly for the
    // create/open call (the bridge marshals them before returning); the
    // manifest is rebuilt from the theory's own tables.
    template<Theory S>
    static auto admit_theory(
        std::expected<foreign::db_handle, foreign::error_handle> opened,
        S const& theory) -> std::expected<Db, Error> {
        return std::move(opened)
            .transform([&theory](foreign::db_handle handle) {
                return Db{
                    std::move(handle), detail::manifest_of(theory)};
            })
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

public:
    /// Creates a fresh DURABLE store (pre-schema lane — module comment).
    static auto create(std::string_view path,
        foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
        return admit(foreign::db_handle::create(path, spec), spec);
    }

    /// Opens an existing durable store, fingerprint-verified.
    static auto open(std::string_view path,
        foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
        return admit(foreign::db_handle::open(path, spec), spec);
    }

    /// Opens or initializes an EPHEMERAL store.
    static auto ephemeral(std::string_view path,
        foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
        return admit(foreign::db_handle::ephemeral(path, spec), spec);
    }

    /// Creates a fresh DURABLE store from a bdb::schema<> value (the
    /// schema lane, TODO_CPP §13): the spec views are built from the
    /// schema's flattened tables — DECLARED statements only, newtype
    /// slots fed from the law-computed class map — and handed to the
    /// engine's SchemaSpec::descriptor(), which stays authoritative.
    template<Theory S>
    static auto create(std::string_view path, S const& theory)
        -> std::expected<Db, Error> {
        auto const spec = foreign::owned_schema_spec{
            detail::owned_relations_of(theory),
            detail::owned_statements_of(theory)};
        return admit_theory(
            foreign::db_handle::create(path, spec.view()), theory);
    }

    /// Opens an existing durable store against a schema value,
    /// fingerprint-verified by the engine.
    template<Theory S>
    static auto open(std::string_view path, S const& theory)
        -> std::expected<Db, Error> {
        auto const spec = foreign::owned_schema_spec{
            detail::owned_relations_of(theory),
            detail::owned_statements_of(theory)};
        return admit_theory(
            foreign::db_handle::open(path, spec.view()), theory);
    }

    /// Opens or initializes an EPHEMERAL store from a schema value.
    template<Theory S>
    static auto ephemeral(std::string_view path, S const& theory)
        -> std::expected<Db, Error> {
        auto const spec = foreign::owned_schema_spec{
            detail::owned_relations_of(theory),
            detail::owned_statements_of(theory)};
        return admit_theory(
            foreign::db_handle::ephemeral(path, spec.view()), theory);
    }

    Db(Db const&) = delete;
    auto operator=(Db const&) -> Db& = delete;
    Db(Db&&) noexcept = default;
    auto operator=(Db&&) noexcept -> Db& = default;
    ~Db() = default;

    /// Whether this handle still owns a store (false after move-out —
    /// the §36 inert-source witness).
    [[nodiscard]] auto alive() const -> bool {
        return handle_.alive();
    }

    /// The admitted store's schema fingerprint: 64 lowercase hex chars
    /// (§33's parity readback).
    [[nodiscard]] auto fingerprint() const
        -> std::expected<std::string, Error> {
        return handle_.fingerprint().transform_error(
            [](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }

    /// Runs the body over one consistent read snapshot (§16),
    /// synchronously on this thread. The body's own typed failure comes
    /// back out through the expected; the Snapshot dies with the callback.
    template<ReadBody Body>
    auto read(Body&& body) const -> std::invoke_result_t<Body&, Snapshot&> {
        using Result = std::invoke_result_t<Body&, Snapshot&>;
        auto slot = std::optional<Result>{};
        auto outcome = handle_.read(
            [&](foreign::bdb_snapshot_ref const& raw)
                -> foreign::bdb_callback_control {
                auto snapshot = Snapshot{raw, manifest_};
                slot.emplace(body(snapshot));
                return slot->has_value()
                    ? foreign::bdb_callback_control::BDB_CALLBACK_CONTROL_OK
                    : foreign::bdb_callback_control::
                          BDB_CALLBACK_CONTROL_ABORT;
            });
        if (!outcome.has_value()) {
            return Result{
                std::unexpect, detail::lift(std::move(outcome).error())};
        }
        contract_assert(slot.has_value());
        return std::move(*slot);
    }

    /// Runs the body as the single writer (§17/§19). Returns the §19
    /// outcome algebra: Committed | Abandoned on success, engine failure
    /// (commit rejection included) as the error. Re-entrant writes are
    /// refused with a typed EnvironmentLocked error.
    template<WriteBody Body>
    auto write(Body&& body) -> typename detail::WriteShape<Body>::Result {
        return write_through(body,
            [this](auto& shim) { return handle_.write(shim); });
    }

    /// write conditional on a still-live snapshot (§18) — legal from
    /// inside the read callback that owns it. A state-changing commit
    /// since the snapshot is the typed GenerationMoved error; retry is
    /// host policy.
    template<WriteBody Body>
    auto write_from(Snapshot& snapshot, Body&& body) ->
        typename detail::WriteShape<Body>::Result {
        return write_through(body, [this, &snapshot](auto& shim) {
            return handle_.write_from(snapshot.raw_, shim);
        });
    }

    /// The witnessed write loop (§18; the TS db.writeWitnessed): one
    /// callback receives a consistent snapshot AND the write tx; the
    /// commit lands only if the generation the snapshot witnessed is
    /// still current. On GenerationMoved the STALE diff is dropped —
    /// never replayed — and the callback reruns against a FRESH snapshot,
    /// up to `witnessed_attempt_cap` attempts; past the cap the typed
    /// WitnessedLivelock refusal comes back (the callback itself moves
    /// the generation each try — host pathology, not engine judgment).
    /// Every other engine failure (commit rejection included) surfaces
    /// unchanged on the first occurrence.
    template<WitnessedBody Body>
    auto write_witnessed(Body&& body) ->
        typename detail::WitnessedShape<Body>::Result {
        using Shape = detail::WitnessedShape<Body>;
        using Result = typename Shape::Result;
        using Outcome = typename Shape::Outcome;
        for (auto attempt = std::uint64_t{1};; ++attempt) {
            auto tried =
                read([&](Snapshot& snapshot)
                        -> std::expected<Outcome, Error> {
                    return write_from(snapshot, [&](WriteTx& tx) {
                        return body(snapshot, tx);
                    });
                });
            if (tried.has_value()) {
                return Result{std::move(*tried)};
            }
            auto error = std::move(tried).error();
            if (error.kind() != ErrorKind::GenerationMoved) {
                return Result{std::unexpect,
                    WitnessedFailure{
                        std::in_place_type<Error>, std::move(error)}};
            }
            if (attempt == witnessed_attempt_cap) {
                return Result{std::unexpect,
                    WitnessedFailure{std::in_place_type<WitnessedLivelock>,
                        WitnessedLivelock{
                            .attempts = attempt,
                            .last = std::move(error),
                        }}};
            }
            // Rebuild on a fresh snapshot (the loop's next read).
        }
    }

    /// Full-relation export, one call (the TS db.scan symmetry): opens a
    /// read snapshot for exactly this scan; the RowSet is owned and
    /// outlives it.
    template<class Facade>
    [[nodiscard]] auto scan(Facade const& relation) const
        -> std::expected<RowSet, Error> {
        return read([&](Snapshot& snapshot)
                -> std::expected<RowSet, Error> {
            return snapshot.scan(relation);
        });
    }

    /// Executes a prepared query, one call (the TS db.execute symmetry):
    /// opens a read snapshot for exactly this execution.
    template<auto Query>
    [[nodiscard]] auto execute(Prepared<Query>& prepared,
        params_of<Query> const& params) const
        -> std::expected<Answers<Query>, Error> {
        return read([&](Snapshot& snapshot)
                -> std::expected<Answers<Query>, Error> {
            return snapshot.execute(prepared, params);
        });
    }

    /// Committed-state keyed point read (§26), one call: opens a read
    /// snapshot for exactly this lookup. The stored key law value is the
    /// selector; the RowSet is owned and outlives the snapshot.
    template<class Facade, class First, class... Rest>
    [[nodiscard]] auto get(Facade const& relation,
        key_law<First, Rest...> const& law,
        typename key_law<First, Rest...>::pattern const& key) const
        -> std::expected<std::optional<RowSet>, Error> {
        return read([&](Snapshot& snapshot)
                -> std::expected<std::optional<RowSet>, Error> {
            return snapshot.get(relation, law, key);
        });
    }

    /// The fresh-field primary read (§26): `db.get(Service, {.id = id})`.
    template<class Facade>
        requires (fresh_field_count<Facade>() >= 1)
    [[nodiscard]] auto get(Facade const& relation,
        fresh_pattern_of<Facade> const& key) const
        -> std::expected<std::optional<RowSet>, Error> {
        return read([&](Snapshot& snapshot)
                -> std::expected<std::optional<RowSet>, Error> {
            return snapshot.get(relation, key);
        });
    }

    /// Prepares one compile-time query value against this store
    /// (TODO_CPP §20, §43: `db.prepare<DownAt>()`). The query already
    /// lowered to a static program-IR view graph during constant
    /// evaluation; the engine's IR validator remains the trust boundary
    /// here — compile-time validation supplements it, never replaces it
    /// (§11).
    template<auto Query>
    [[nodiscard]] auto prepare() const
        -> std::expected<Prepared<Query>, Error> {
        return handle_.prepare(foreign::program_of<Query>)
            .transform([](foreign::prepared_handle handle) {
                return Prepared<Query>{std::move(handle)};
            })
            .transform_error([](foreign::error_handle handle) {
                return detail::lift(std::move(handle));
            });
    }
};

} // namespace bdb
