// :schema — the consteval schema elaborator and the schema value
// (TODO_CPP §9–§10, §26; lowering.md §2–§3).
//
// Statements are VALUES (laws are first-class, §26): `bdb::key(...)`,
// `bdb::contained(...)`, `bdb::mirrors(...)`, `bdb::capacity(...)`
// construct literal structural values whose IDENTITY rides their types
// (the coordinates carry relation/field/kind/fresh as NTTPs), so
// `bdb::schema<"Uptime">(...)` runs the whole class-law computation — the
// union-find of lowering.md §3 — during constant evaluation, with §34
// diagnostics that name semantic coordinates.
//
// The schema value is an NTTP-friendly literal: it exposes its relations
// as members (`Uptime.relations.Service.id`), the flattened SchemaSpec
// data the runtime lane lowers to the bridge (relations/fields in
// declaration order, DECLARED statements only in written order —
// lowering.md §2/§7), and the law-computed class map (a consteval lookup;
// class names feed newtype slots ONLY and never move the fingerprint —
// lowering.md §1.10, §3.7).
export module bumbledb:schema;

import std;
import :name;
import :classify;
import :spec;
import :schema_member;
import :closed_facade;
import :contained;
import :capacity;
import :classes;

namespace bdb::detail {

// ————————————————————————————————————————————————————————————————————
// The relations-as-members injection.
// ————————————————————————————————————————————————————————————————————

template<class... Args>
struct schema_relation_types {
    struct Relations;
    consteval {
        auto specs = std::vector<std::meta::info>{};
        [[maybe_unused]] auto const add = [&]<class A>() {
            // The branch must be constexpr: the name reader may only be
            // instantiated for member types.
            if constexpr (is_member_type(^^A)) {
                // Skip a duplicate name so the injection stays total and
                // schema()'s static_assert carries the one diagnostic.
                auto const name = facade_relation_name_of<A>();
                for (auto const spec : specs) {
                    if (std::meta::identifier_of(spec) == name.view()) {
                        return;
                    }
                }
                specs.push_back(std::meta::data_member_spec(
                    ^^A, {.name = spec_name(name.view())}));
            }
        };
        (add.template operator()<Args>(), ...);
        std::meta::define_aggregate(^^Relations, specs);
    }
};

// The fresh-pattern injection (§26's primary lane): one member per
// fresh-marked coordinate of the facade.
template<class Facade>
struct fresh_pattern_types {
    struct Pattern;
    consteval {
        auto specs = std::vector<std::meta::info>{};
        for (auto const member : std::meta::nonstatic_data_members_of(
                 ^^Facade, std::meta::access_context::current())) {
            auto const args = std::meta::template_arguments_of(
                std::meta::dealias(std::meta::type_of(member)));
            if (std::meta::extract<bool>(args[5])) {
                specs.push_back(std::meta::data_member_spec(args[0],
                    {.name = spec_name(
                         std::meta::extract<name_text>(args[2]).view())}));
            }
        }
        std::meta::define_aggregate(^^Pattern, specs);
    }
};

} // namespace bdb::detail

export namespace bdb {

/// The fresh-field primary read pattern of a facade (§26):
/// `db.get(Service, {.id = id})`.
template<class Facade>
using fresh_pattern_of =
    typename detail::fresh_pattern_types<Facade>::Pattern;

/// The facade's fresh-field count (constrains the primary get lane).
template<class Facade>
consteval auto fresh_field_count() -> std::size_t {
    auto const entry = detail::relation_entry<Facade>();
    auto count = std::size_t{0};
    for (auto field = std::size_t{0}; field != entry.field_count;
        ++field) {
        if (entry.fields[field].fresh) {
            ++count;
        }
    }
    return count;
}

// ————————————————————————————————————————————————————————————————————
// The schema value.
// ————————————————————————————————————————————————————————————————————

/// A whole theory as one literal value (TODO_CPP §9–§10): relations as
/// members (argument order = declaration order = RelationId mint), the
/// flattened SchemaSpec data (DECLARED statements only, written order),
/// and the law-computed class map with a consteval lookup. Structural —
/// a schema travels as a template argument.
template<fixed_string Name, class... Args>
struct schema_value {
    static constexpr std::size_t relation_count =
        detail::relation_count<Args...>();
    static constexpr std::size_t statement_count =
        detail::statement_count<Args...>();
    static constexpr std::size_t coordinate_count =
        detail::coord_count<Args...>();

    /// The schema's name (diagnostics only; never on the wire).
    name_text schema_name;

    /// The schema's name at the TYPE tier (query-layer diagnostics run in
    /// contexts where only template arguments are constant-visible).
    static constexpr auto declared_name = Name;

    /// The relation facades, one member per relation, named identically —
    /// `Uptime.relations.Service.id` (the later query surface).
    typename detail::schema_relation_types<Args...>::Relations relations;

    /// Relations/fields in declaration order — the wire lane's source.
    std::array<relation_data, relation_count> relation_table;

    /// DECLARED statements only, written order (lowering.md §2.1/§7.1).
    std::array<statement_data, statement_count> statements;

    /// The class map: one entry per coordinate in relation-declaration ×
    /// field-declaration order (lowering.md §3).
    std::array<class_entry, coordinate_count> classes;

    /// Type-derivable twins of the value tables: the query elaborator
    /// (the query partitions) recomputes the flattened relation table and
    /// the class map from the schema TYPE during rule elaboration — the
    /// static_assert walls there can only read template arguments, never
    /// a schema VALUE. Deterministic: schema() fills the members from
    /// exactly these computations.
    static consteval auto member_relation_table()
        -> std::array<relation_data, relation_count> {
        return detail::relation_table<Args...>();
    }

    static consteval auto member_class_map()
        -> std::array<class_entry, coordinate_count> {
        return detail::analyze_schema<Args...>().classes;
    }

    /// The consteval class lookup: a coordinate's law-computed class
    /// identity (nullopt = bare). The class NAME is the returned
    /// coordinate rendered "Relation.field".
    template<class Coordinate>
    [[nodiscard]] consteval auto class_of(Coordinate) const
        -> std::optional<coord_ref> {
        static_assert(detail::is_coordinate_v<Coordinate>,
            "bumbledb schema::class_of(): the argument must be a relation "
            "coordinate (Relation.field)");
        for (auto const& entry : classes) {
            if (entry.coordinate.relation == Coordinate::relation_name
                && entry.coordinate.field == Coordinate::field_name) {
                if (!entry.classed) {
                    return std::nullopt;
                }
                return entry.class_name;
            }
        }
        return std::nullopt;
    }
};

/// The consteval schema elaborator (TODO_CPP §9–§10): relations first (in
/// declaration order), then statements (in written order). Runs the class
/// laws and every structurally decidable §34 wall; hands NOTHING semantic
/// to the wire beyond names — the engine's SchemaSpec::descriptor()
/// remains the authority.
template<fixed_string Name, class... Args>
consteval auto schema(Args const&... args) -> schema_value<Name, Args...> {
    static_assert(detail::args_recognized<Args...>(),
        "bumbledb schema(): every argument must be a relation facade "
        "(bdb::relation<...>) or a statement value (bdb::key / "
        "bdb::contained / bdb::mirrors / bdb::capacity)");
    static_assert(detail::relation_count<Args...>() > 0,
        "bumbledb schema(): a schema declares at least one relation");
    static_assert(detail::relations_lead<Args...>(),
        "bumbledb schema(): relations precede statements — declare every "
        "member facade before the first statement");
    static_assert(detail::relation_names_distinct<Args...>(),
        "bumbledb schema(): two member relations share one name");

    static_assert(detail::analyze_schema<Args...>().members_known,
        detail::membership_message<Args...>(Name.view()));
    static_assert(detail::analyze_schema<Args...>().lawful,
        detail::wall_message<Args...>(Name.view()));
    static_assert(
        detail::analyze_schema<Args...>().no_restated_implied_key,
        detail::restated_key_message<Args...>(Name.view()));
    static_assert(detail::analyze_schema<Args...>().no_duplicate_key,
        detail::duplicate_key_message<Args...>(Name.view()));

    auto out = schema_value<Name, Args...>{
        .schema_name = detail::to_name_text(Name.view()),
        .relations = {},
        .relation_table = detail::relation_table<Args...>(),
        .statements = detail::statement_shapes<Args...>(),
        .classes = detail::analyze_schema<Args...>().classes,
    };

    // The value-borne payloads: capacity windows, face σ/ψ selections,
    // and closed relations' sealed extensions (axiom values live on the
    // facade VALUE — the type tier carries only the sealed roster).
    auto relation_index = std::size_t{0};
    auto index = std::size_t{0};
    auto const copy_selection = [](side_data& side, auto const& from) {
        side.selection_count = from.selection_count;
        side.selections = from.selections;
    };
    auto const fill = [&]<class A>(A const& argument) {
        if constexpr (detail::is_statement_v<A>) {
            if constexpr (detail::is_capacity_v<A>) {
                out.statements[index].window = argument.window;
                copy_selection(
                    out.statements[index].target, argument.target);
                copy_selection(
                    out.statements[index].source, argument.source);
            }
            if constexpr (detail::is_containment_v<A>) {
                copy_selection(
                    out.statements[index].source, argument.source);
                copy_selection(
                    out.statements[index].target, argument.target);
            }
            ++index;
        } else {
            if constexpr (detail::is_closed_facade<A>()) {
                out.relation_table[relation_index].closed_data =
                    argument.data;
            }
            ++relation_index;
        }
    };
    (fill(args), ...);
    return out;
}

/// The schema concept the runtime lane (the db partitions) admits: any
/// literal value carrying the flattened tables (structural, never
/// nominal).
template<class S>
concept Theory = requires(S const& theory) {
    requires std::same_as<std::ranges::range_value_t<
                              decltype(theory.relation_table)>,
        relation_data>;
    requires std::same_as<
        std::ranges::range_value_t<decltype(theory.statements)>,
        statement_data>;
    requires std::same_as<
        std::ranges::range_value_t<decltype(theory.classes)>,
        class_entry>;
};

} // namespace bdb
