// bumbledb.meta.query — the coordinate-typed query builder and its
// compile-time IR lowering (TODO_CPP §11–§12, §21; lowering.md §4).
//
// GCC-only reflection zone. The semantic reference is the TypeScript query
// builder (ts/src/query/): `bdb::query(Schema).rule([](auto r) consteval
// {...})` mirrors `query(Schema).rule((r) => ...)` — vars / match / where /
// find / param — and the rule's builder value lowers DURING CONSTANT
// EVALUATION to the flattened program IR (`query_ir`) that
// bumbledb.foreign.program later presents to the bridge as static
// `bdb_program` views, exactly as ts/src/query/lower.ts builds them:
// per-rule dense variable ids by first occurrence over the written walk
// (body items in written order — EDB bindings in written property order at
// field ordinals — then the finds last), the param registry in first-use
// order (registry order = positional bind order), and the one-predicate
// program with output 0 (lowering.md §4.2, §7.13–§7.14).
//
// Variables are minted per relation column by `r.vars(Relation)` — a
// synthesized product (define_aggregate) with one member per field, named
// identically. MEMBER ACCESS IS THE ONLY SUPPORTED BINDING, deliberately:
// the TS SDK's destructuring is NAMED, while C++ structured bindings are
// positional — a weaker thing that hides the field name at the binding
// site — so nothing here enables the tuple protocol (TODO_CPP §11).
// Variable IDENTITY is the mint coordinate carried in the variable's TYPE
// (the C++ image of the TS object-reference identity: one schema mints one
// variable per coordinate), and each variable carries its column's
// law-computed class — reusing a variable against a field of another
// semantic class fails constant evaluation with a diagnostic naming BOTH
// coordinates (§34; two physical u64 columns are not query-compatible
// merely because both are uint64_t).
//
// Phase scope (recipe-parity target: cookbook recipe 1): EDB atoms over
// ordinary relations, leaf conditions (eq/ne/lt/le/gt/ge, allen with a
// literal mask, point_in), scalar value params, var finds, and the
// sum(duration) aggregate head. Recs/negation/or-trees/set/mask params
// arrive with later phases; the IR shapes below already carry their slots.
export module bumbledb.meta.query;

import std;
import bumbledb.types;
import bumbledb.meta.relation;
import bumbledb.meta.schema;

export namespace bdb {

// ————————————————————————————————————————————————————————————————————
// Capacities (Phase-D bounds; the engine's own caps are far higher).
// ————————————————————————————————————————————————————————————————————

inline constexpr std::size_t max_query_rules = 4;
inline constexpr std::size_t max_query_atoms = 8;
inline constexpr std::size_t max_query_conditions = 8;
inline constexpr std::size_t max_query_finds = 8;
inline constexpr std::size_t max_query_params = 8;
inline constexpr std::size_t max_query_vars = 32;

// ————————————————————————————————————————————————————————————————————
// The flattened query IR (structural values — a query is NTTP-friendly).
// ————————————————————————————————————————————————————————————————————

/// One structural literal payload (match/comparison literals). Strings
/// and bytes are deliberately absent: a query VALUE must stay structural
/// (NTTP-usable), and no cookbook query literal needs them — bind such
/// values through params instead.
struct query_literal {
    value_kind kind;
    bool boolean;
    std::uint64_t u64;
    std::int64_t i64;
    std::uint64_t u64_start;
    std::uint64_t u64_end;
    std::int64_t i64_start;
    std::int64_t i64_end;
};

/// A term's form (`ir::Term`, lowering.md §4.1). `absent` is the pattern
/// wildcard — an unmentioned field binds nothing.
enum class query_term_form : std::uint8_t {
    absent,
    variable,
    param,
    literal,
    measure,
};

/// One builder-stage term: variables/measures ride their MINT coordinate
/// (the identity `v(Relation).field` established), params their name.
struct term_data {
    query_term_form form;
    coord_ref variable;
    name_text param;
    query_literal literal;
};

/// One pattern binding as recorded: the sealed field ordinal + the term.
struct binding_data {
    std::size_t field;
    term_data term;
};

/// One EDB atom as recorded: the relation's declaration ordinal (the wire
/// RelationId — lowering.md §1.1) and the bindings in written order.
struct atom_data {
    std::uint32_t relation;
    std::size_t binding_count;
    std::array<binding_data, max_relation_fields> bindings;
};

/// The comparison operators the surface mints (`ir::CmpOp`).
enum class query_cmp : std::uint8_t {
    eq,
    ne,
    lt,
    le,
    gt,
    ge,
    allen,
    point_in,
};

/// One leaf condition. `point_in` stores interval-LEFT, point-RIGHT
/// whatever the surface argument order (ts/src/query/atom.ts:432-435);
/// `mask` is the literal 13-bit Allen word (allen conditions only).
struct condition_data {
    query_cmp op;
    std::uint16_t mask;
    term_data lhs;
    term_data rhs;
};

/// One rule-body item: the written interleave of match/where is preserved
/// so variable numbering walks body items in WRITTEN order (lowering.md
/// §4.2), whatever bucket each item later lowers into.
enum class body_form : std::uint8_t {
    atom,
    condition,
};

struct body_item {
    body_form form;
    atom_data atom;
    condition_data condition;
};

/// A find column's form (`ir::FindTerm`).
enum class find_form : std::uint8_t {
    variable,
    aggregate_measure,
};

/// The fold ops the aggregate heads mint (`ir::AggOp` slice).
enum class fold_form : std::uint8_t {
    sum,
    min,
    max,
};

/// One find column: the answer column name, the term shape, and the
/// answer cell's structural class (the row-product synthesis input).
struct find_data {
    name_text name;
    find_form form;
    fold_form op;
    term_data over;
    field_class answer;
};

/// A param's wire shape (lowering.md §4.2's registry entry).
enum class param_shape : std::uint8_t {
    value,
    set,
    mask,
};

/// One registered parameter: name, shape, the field-anchored bind domain
/// (the params-product member type AND the wire tag), and whether the
/// anchoring use was point-domain (an interval field's element under
/// point_in — TODO_CPP §21).
struct param_data {
    name_text name;
    param_shape shape;
    field_class domain;
    bool point;
};

/// One param USE, recorded at the position that anchors it.
struct param_use {
    name_text name;
    param_shape shape;
    field_class domain;
    bool point;
};

/// One rule's accumulated builder state (value tier).
struct rule_state {
    std::size_t item_count;
    std::array<body_item, max_query_atoms + max_query_conditions> items;
    std::size_t use_count;
    std::array<param_use, max_query_params * 4> uses;
    std::size_t bound_count;
    std::array<coord_ref, max_query_vars> bound;
};

/// One completed rule: the body state plus the find head.
struct rule_data {
    rule_state state;
    std::size_t find_count;
    std::array<find_data, max_query_finds> finds;
};

// ————————————————————————————————————————————————————————————————————
// The numbered wire IR (what bumbledb.foreign.program reads).
// ————————————————————————————————————————————————————————————————————

/// One numbered term: dense rule-scoped var ids, dense query-global param
/// ids (registry order = positional bind order — lowering.md §5.1).
struct wire_term {
    query_term_form form;
    std::uint16_t var;
    std::uint16_t param;
    query_literal literal;
};

struct wire_binding {
    std::uint16_t field;
    wire_term term;
};

struct wire_atom {
    std::uint32_t relation;
    std::size_t binding_count;
    std::array<wire_binding, max_relation_fields> bindings;
};

struct wire_condition {
    query_cmp op;
    std::uint16_t mask;
    wire_term lhs;
    wire_term rhs;
};

struct wire_find {
    find_form form;
    fold_form op;
    std::uint16_t over;
};

/// One numbered rule, bucketed exactly as the bridge's `bdb_rule` reads
/// it (atoms / conditions in written order; negation arrives later).
struct wire_rule {
    std::size_t atom_count;
    std::array<wire_atom, max_query_atoms> atoms;
    std::size_t condition_count;
    std::array<wire_condition, max_query_conditions> conditions;
    std::size_t find_count;
    std::array<wire_find, max_query_finds> finds;
};

/// The whole lowered query: the degenerate one-predicate program
/// (`output = 0` — lowering.md §4.1), plus the head columns (row-product
/// synthesis) and the param registry (params-product synthesis).
struct query_ir {
    std::size_t rule_count;
    std::array<wire_rule, max_query_rules> rules;
    std::size_t head_count;
    std::array<find_data, max_query_finds> head;
    std::size_t param_count;
    std::array<param_data, max_query_params> params;
};

// ————————————————————————————————————————————————————————————————————
// Scope terms.
// ————————————————————————————————————————————————————————————————————

/// One query variable, minted by `r.vars(Relation)`: identity and typing
/// live in the TYPE — the mint coordinate, the column's structural class,
/// and the column's law-computed class (the schema's class laws, TODO_CPP
/// §10–§11). Values are empty structural literals.
template<class T, name_text Relation, name_text Field, field_class Class,
    bool Classed, coord_ref Law>
struct qvar {
    using value_type = T;

    static constexpr name_text relation_name = Relation;
    static constexpr name_text field_name = Field;
    static constexpr field_class cls = Class;
    static constexpr bool classed = Classed;
    static constexpr coord_ref law = Law;
};

/// A scalar query parameter — `r.param<"t">()`. The name is the member of
/// the typed params product `execute` takes; the domain is inferred from
/// the first anchored use (a binding's field, or a comparison sibling).
template<fixed_string Name>
struct param_ref {
    static constexpr name_text name = detail::to_name_text(Name.view());
};

/// The measure of an interval variable — `r.duration(vars.window)`:
/// `|[s, e)| = e − s`, u64 (`ir::Term::Measure`).
template<class Var>
struct measure_ref {
    using over = Var;
};

/// One named aggregate head column — `r.sum<"downtime">(r.duration(w))`.
/// The NAME is carried here because a C++ designated-init head cannot
/// mint new member names the way a TS object literal can; the aggregate
/// column's name is the one datum the pattern product cannot express.
template<fixed_string Name, class Var, fold_form Op>
struct fold_ref {
    using over = Var;
    static constexpr name_text column_name = detail::to_name_text(Name.view());
    static constexpr fold_form op = Op;
};

} // namespace bdb

namespace bdb::detail {

// ————————————————————————————————————————————————————————————————————
// Recognition + the shared consteval string helpers (§34 diagnostics).
// ————————————————————————————————————————————————————————————————————

template<class T>
inline constexpr bool is_qvar_v = false;

template<class T, name_text R, name_text F, field_class C, bool Cl,
    coord_ref L>
inline constexpr bool is_qvar_v<qvar<T, R, F, C, Cl, L>> = true;

template<class T>
inline constexpr bool is_param_ref_v = false;

template<fixed_string Name>
inline constexpr bool is_param_ref_v<param_ref<Name>> = true;

template<class T>
inline constexpr bool is_measure_ref_v = false;

template<class Var>
inline constexpr bool is_measure_ref_v<measure_ref<Var>> = true;

consteval auto is_query_coord_type(std::meta::info type) -> bool {
    auto const t = std::meta::dealias(type);
    return std::meta::has_template_arguments(t)
        && std::meta::template_of(t) == ^^coord;
}

/// A relation facade: a class whose every member is a coordinate (the
/// injected Coords product of bumbledb.meta.relation).
template<class Facade>
consteval auto is_query_facade() -> bool {
    auto const t = std::meta::dealias(^^Facade);
    if (!std::meta::is_class_type(t)) {
        return false;
    }
    auto const members = std::meta::nonstatic_data_members_of(
        t, std::meta::access_context::current());
    if (members.empty()) {
        return false;
    }
    for (auto const member : members) {
        if (!is_query_coord_type(std::meta::type_of(member))) {
            return false;
        }
    }
    return true;
}

template<class Facade>
consteval auto facade_relation_name() -> name_text {
    auto const members = std::meta::nonstatic_data_members_of(
        ^^Facade, std::meta::access_context::current());
    auto const args = std::meta::template_arguments_of(
        std::meta::dealias(std::meta::type_of(members[0])));
    return std::meta::extract<name_text>(args[1]);
}

consteval auto render_size(std::size_t value) -> std::string {
    if (value == 0) {
        return "0";
    }
    auto out = std::string{};
    while (value != 0) {
        out.insert(out.begin(),
            static_cast<char>('0' + static_cast<char>(value % 10)));
        value /= 10;
    }
    return out;
}

consteval auto coordinate_label(name_text relation, name_text field)
    -> std::string {
    return std::string{relation.view()} + "." + std::string{field.view()};
}

consteval auto kind_label(field_class cls) -> std::string {
    switch (cls.kind) {
    case value_kind::boolean:
        return "bool";
    case value_kind::u64:
        return "u64";
    case value_kind::i64:
        return "i64";
    case value_kind::string:
        return "str";
    case value_kind::fixed_bytes:
        return "bytes<" + render_size(cls.fixed_len) + ">";
    case value_kind::interval_u64:
        return "interval<u64>";
    case value_kind::interval_i64:
        break;
    }
    return "interval<i64>";
}

consteval auto class_label(bool classed, coord_ref law) -> std::string {
    if (!classed) {
        return "bare";
    }
    return "class \"" + coordinate_label(law.relation, law.field) + "\"";
}

/// The §34 cross-class wall: names BOTH coordinates and both classes.
template<name_text VarRel, name_text VarField, bool VarClassed,
    coord_ref VarLaw, name_text AtRel, name_text AtField, bool AtClassed,
    coord_ref AtLaw>
consteval auto cross_class_message(std::string_view verb) -> std::string {
    return "bumbledb query: variable \""
        + coordinate_label(VarRel, VarField) + "\" ("
        + class_label(VarClassed, VarLaw) + ") cannot " + std::string{verb}
        + " coordinate \"" + coordinate_label(AtRel, AtField) + "\" ("
        + class_label(AtClassed, AtLaw)
        + ") — a variable joins only class-equal columns (one variable, "
          "one law class)";
}

/// The physical-kind wall (structurally unequal columns).
template<name_text VarRel, name_text VarField, field_class VarClass,
    name_text AtRel, name_text AtField, field_class AtClass>
consteval auto kind_mismatch_message(std::string_view verb) -> std::string {
    return "bumbledb query: variable \""
        + coordinate_label(VarRel, VarField) + "\" ("
        + kind_label(VarClass) + ") cannot " + std::string{verb}
        + " coordinate \"" + coordinate_label(AtRel, AtField) + "\" ("
        + kind_label(AtClass) + ") — the structural kinds differ";
}

template<class S, class Facade>
consteval auto foreign_relation_message() -> std::string {
    return std::string{"bumbledb query over schema \""}
        + std::string{S::declared_name.view()} + "\": relation \""
        + std::string{facade_relation_name<Facade>().view()}
        + "\" is not a member of the schema";
}

// The law-class lookup off the schema TYPE.
template<class S>
consteval auto law_of(name_text relation, name_text field)
    -> std::pair<bool, coord_ref> {
    for (auto const& entry : S::member_class_map()) {
        if (entry.coordinate.relation == relation
            && entry.coordinate.field == field) {
            return {entry.classed, entry.class_name};
        }
    }
    return {false, coord_ref{}};
}

inline constexpr std::size_t no_relation = ~std::size_t{0};

template<class S, class Facade>
consteval auto relation_ordinal() -> std::size_t {
    auto const table = S::member_relation_table();
    auto const relation = facade_relation_name<Facade>();
    for (auto index = std::size_t{0}; index != table.size(); ++index) {
        if (table[index].name == relation) {
            return index;
        }
    }
    return no_relation;
}

template<class S, class Facade>
consteval auto facade_in_schema() -> bool {
    return relation_ordinal<S, Facade>() != no_relation;
}

// ————————————————————————————————————————————————————————————————————
// Value-tier walls (the bumbledb.types diagnostic convention: reaching a
// call to a never-defined non-constexpr function during constant
// evaluation is the compile error, and the name is the message).
// ————————————————————————————————————————————————————————————————————

auto query_has_too_many_rules() -> void;
auto rule_has_too_many_atoms() -> void;
auto rule_has_too_many_conditions() -> void;
auto rule_has_too_many_variables() -> void;
auto rule_has_too_many_finds() -> void;
auto rule_finds_nothing() -> void;
auto query_has_too_many_params() -> void;
auto query_param_is_used_at_two_shapes() -> void;
auto query_param_is_inferred_inconsistently_across_uses() -> void;
auto where_condition_variable_is_not_bound_in_this_rule() -> void;
auto find_head_variable_is_not_bound_in_this_rule() -> void;
auto find_head_names_must_be_distinct() -> void;
auto every_rule_of_a_query_must_derive_the_same_head() -> void;

// ————————————————————————————————————————————————————————————————————
// Term helpers.
// ————————————————————————————————————————————————————————————————————

template<class Var>
consteval auto var_term() -> term_data {
    return term_data{
        .form = query_term_form::variable,
        .variable = coord_ref{
            .relation = Var::relation_name, .field = Var::field_name},
        .param = name_text{},
        .literal = query_literal{},
    };
}

template<class Var>
consteval auto measure_term() -> term_data {
    auto out = var_term<Var>();
    out.form = query_term_form::measure;
    return out;
}

template<class Param>
consteval auto param_term() -> term_data {
    return term_data{
        .form = query_term_form::param,
        .variable = coord_ref{},
        .param = Param::name,
        .literal = query_literal{},
    };
}

consteval auto literal_term(query_literal literal) -> term_data {
    return term_data{
        .form = query_term_form::literal,
        .variable = coord_ref{},
        .param = name_text{},
        .literal = literal,
    };
}

/// Tags one integral host literal at a scalar domain (the sibling/field
/// directs the tag — lowering.md §4.2's field/sibling-directed tagging).
template<class T>
consteval auto scalar_literal(value_kind kind, T value) -> query_literal {
    auto out = query_literal{};
    out.kind = kind;
    if (kind == value_kind::boolean) {
        out.boolean = static_cast<bool>(value);
    } else if (kind == value_kind::u64) {
        out.u64 = static_cast<std::uint64_t>(value);
    } else {
        out.i64 = static_cast<std::int64_t>(value);
    }
    return out;
}

consteval auto interval_literal(interval<std::uint64_t> value)
    -> query_literal {
    auto out = query_literal{};
    out.kind = value_kind::interval_u64;
    out.u64_start = value.lo();
    out.u64_end = value.hi();
    return out;
}

consteval auto interval_literal(interval<std::int64_t> value)
    -> query_literal {
    auto out = query_literal{};
    out.kind = value_kind::interval_i64;
    out.i64_start = value.lo();
    out.i64_end = value.hi();
    return out;
}

consteval auto add_use(rule_state& state, param_use use) -> void {
    if (state.use_count == state.uses.size()) {
        query_has_too_many_params();
    }
    state.uses[state.use_count] = use;
    ++state.use_count;
}

consteval auto add_bound(rule_state& state, coord_ref variable) -> void {
    for (auto index = std::size_t{0}; index != state.bound_count; ++index) {
        if (state.bound[index] == variable) {
            return;
        }
    }
    if (state.bound_count == max_query_vars) {
        rule_has_too_many_variables();
    }
    state.bound[state.bound_count] = variable;
    ++state.bound_count;
}

consteval auto is_bound(rule_state const& state, coord_ref variable)
    -> bool {
    for (auto index = std::size_t{0}; index != state.bound_count; ++index) {
        if (state.bound[index] == variable) {
            return true;
        }
    }
    return false;
}

} // namespace bdb::detail

export namespace bdb {

/// One built condition value (a `.where` argument): the leaf comparison
/// plus the param uses its construction anchored.
struct cond_value {
    condition_data data;
    std::size_t use_count;
    std::array<param_use, 2> uses;
};

} // namespace bdb

namespace bdb::detail {

// ————————————————————————————————————————————————————————————————————
// Pattern-product slots. Identity rides the slot TYPE (coordinate +
// classes as NTTPs) so the converting constructors can run the §34
// class walls as static_asserts naming semantic coordinates.
// ————————————————————————————————————————————————————————————————————

/// One match-pattern slot: default state is the wildcard (an unmentioned
/// designated-init member binds nothing — `ir::Atom`'s absence rule).
template<class T, name_text Relation, name_text Field, std::size_t Ordinal,
    field_class Class, bool Classed, coord_ref Law>
struct binding_slot {
    static constexpr name_text relation_name = Relation;
    static constexpr name_text field_name = Field;
    static constexpr std::size_t ordinal = Ordinal;
    static constexpr field_class cls = Class;

    term_data term{};

    binding_slot() = default;

    /// A variable binding: the class walls run HERE, where both the
    /// variable's mint slot and the field's slot are template-visible.
    template<class VT, name_text VR, name_text VF, field_class VC, bool VCl,
        coord_ref VLaw>
    consteval binding_slot(qvar<VT, VR, VF, VC, VCl, VLaw> variable) {
        static_assert(VC == Class,
            kind_mismatch_message<VR, VF, VC, Relation, Field, Class>(
                "bind"));
        static_assert(VCl == Classed && (!Classed || VLaw == Law),
            cross_class_message<VR, VF, VCl, VLaw, Relation, Field, Classed,
                Law>("bind"));
        term = var_term<decltype(variable)>();
    }

    /// A scalar param binding, anchored at this field's domain.
    template<fixed_string Name>
    consteval binding_slot(param_ref<Name> parameter) {
        term = param_term<decltype(parameter)>();
    }

    /// A bare literal at a fixed-width field (field-directed tagging).
    consteval binding_slot(T value)
        requires (Class.kind == value_kind::boolean
            || Class.kind == value_kind::u64
            || Class.kind == value_kind::i64)
    {
        term = literal_term(scalar_literal(Class.kind, value));
    }

    /// An interval literal at an interval field.
    consteval binding_slot(T value)
        requires (Class.kind == value_kind::interval_u64
            || Class.kind == value_kind::interval_i64)
    {
        term = literal_term(interval_literal(value));
    }
};

/// One find-pattern slot: accepts a class-equal bound variable only (the
/// boundness wall is value-tier, judged at rule assembly).
template<class T, name_text Relation, name_text Field, field_class Class,
    bool Classed, coord_ref Law>
struct find_slot {
    static constexpr name_text field_name = Field;
    static constexpr field_class cls = Class;

    term_data term{};

    find_slot() = default;

    template<class VT, name_text VR, name_text VF, field_class VC, bool VCl,
        coord_ref VLaw>
    consteval find_slot(qvar<VT, VR, VF, VC, VCl, VLaw> variable) {
        static_assert(VC == Class,
            kind_mismatch_message<VR, VF, VC, Relation, Field, Class>(
                "answer at"));
        static_assert(VCl == Classed && (!Classed || VLaw == Law),
            cross_class_message<VR, VF, VCl, VLaw, Relation, Field, Classed,
                Law>("answer at"));
        term = var_term<decltype(variable)>();
    }
};

// ————————————————————————————————————————————————————————————————————
// Product synthesis (the proven class-template-scope define_aggregate
// pattern, TODO_CPP §38). Facade coordinate args: 0 = value type,
// 1 = relation, 2 = field, 3 = ordinal, 4 = field_class, 5 = fresh.
// ————————————————————————————————————————————————————————————————————

template<class S, class Facade>
struct rule_vars_types {
    struct Vars;
    consteval {
        auto specs = std::vector<std::meta::info>{};
        for (auto const member : std::meta::nonstatic_data_members_of(
                 ^^Facade, std::meta::access_context::current())) {
            auto const args = std::meta::template_arguments_of(
                std::meta::dealias(std::meta::type_of(member)));
            auto const law = law_of<S>(std::meta::extract<name_text>(args[1]),
                std::meta::extract<name_text>(args[2]));
            specs.push_back(std::meta::data_member_spec(
                std::meta::substitute(^^qvar,
                    {args[0], args[1], args[2], args[4],
                        std::meta::reflect_constant(law.first),
                        std::meta::reflect_constant(law.second)}),
                {.name = std::meta::identifier_of(member)}));
        }
        std::meta::define_aggregate(^^Vars, specs);
    }
};

template<class S, class Facade>
struct match_pattern_types {
    struct Pattern;
    consteval {
        auto specs = std::vector<std::meta::info>{};
        for (auto const member : std::meta::nonstatic_data_members_of(
                 ^^Facade, std::meta::access_context::current())) {
            auto const args = std::meta::template_arguments_of(
                std::meta::dealias(std::meta::type_of(member)));
            auto const law = law_of<S>(std::meta::extract<name_text>(args[1]),
                std::meta::extract<name_text>(args[2]));
            specs.push_back(std::meta::data_member_spec(
                std::meta::substitute(^^binding_slot,
                    {args[0], args[1], args[2], args[3], args[4],
                        std::meta::reflect_constant(law.first),
                        std::meta::reflect_constant(law.second)}),
                {.name = std::meta::identifier_of(member)}));
        }
        std::meta::define_aggregate(^^Pattern, specs);
    }
};

/// The find pattern spans every matched relation's coordinates, in match
/// order, first name wins on a collision (single-relation rules — the
/// recipe shape — never collide).
template<class S, class... Facades>
struct find_pattern_types {
    struct Pattern;
    consteval {
        auto specs = std::vector<std::meta::info>{};
        auto taken = std::vector<std::string>{};
        [[maybe_unused]] auto const add = [&]<class Facade>() {
            for (auto const member : std::meta::nonstatic_data_members_of(
                     ^^Facade, std::meta::access_context::current())) {
                auto const name =
                    std::string{std::meta::identifier_of(member)};
                auto duplicate = false;
                for (auto const& seen : taken) {
                    if (seen == name) {
                        duplicate = true;
                    }
                }
                if (duplicate) {
                    continue;
                }
                taken.push_back(name);
                auto const args = std::meta::template_arguments_of(
                    std::meta::dealias(std::meta::type_of(member)));
                auto const law =
                    law_of<S>(std::meta::extract<name_text>(args[1]),
                        std::meta::extract<name_text>(args[2]));
                specs.push_back(std::meta::data_member_spec(
                    std::meta::substitute(^^find_slot,
                        {args[0], args[1], args[2], args[4],
                            std::meta::reflect_constant(law.first),
                            std::meta::reflect_constant(law.second)}),
                    {.name = std::meta::identifier_of(member)}));
            }
        };
        (add.template operator()<Facades>(), ...);
        std::meta::define_aggregate(^^Pattern, specs);
    }
};

} // namespace bdb::detail

export namespace bdb {

/// The synthesized variable product of one relation under one schema:
/// one member per field, named identically, each a `bdb::qvar` carrying
/// the coordinate and its law class. MEMBER ACCESS ONLY (module comment).
template<class S, class Facade>
using vars_of = typename detail::rule_vars_types<S, Facade>::Vars;

/// The designated-init match pattern of one relation under one schema.
template<class S, class Facade>
using match_pattern_of =
    typename detail::match_pattern_types<S, Facade>::Pattern;

/// The designated-init find head over the rule's matched relations.
template<class S, class... Facades>
using find_pattern_of =
    typename detail::find_pattern_types<S, Facades...>::Pattern;

} // namespace bdb

namespace bdb::detail {

// ————————————————————————————————————————————————————————————————————
// Recording (value tier).
// ————————————————————————————————————————————————————————————————————

template<class S, class Facade>
consteval auto record_match(rule_state& state,
    match_pattern_of<S, Facade> const& pattern) -> void {
    if (state.item_count == state.items.size()) {
        rule_has_too_many_atoms();
    }
    auto atom = atom_data{};
    atom.relation =
        static_cast<std::uint32_t>(relation_ordinal<S, Facade>());

    using Pattern = match_pattern_of<S, Facade>;
    constexpr auto members = std::define_static_array(
        std::meta::nonstatic_data_members_of(
            ^^Pattern, std::meta::access_context::current()));
    template for (constexpr auto index : index_array<members.size()>()) {
        using Slot = [:std::meta::type_of(members[index]):];
        auto const& slot = pattern.[:members[index]:];
        if (slot.term.form == query_term_form::absent) {
            continue;
        }
        atom.bindings[atom.binding_count] = binding_data{
            .field = Slot::ordinal,
            .term = slot.term,
        };
        ++atom.binding_count;
        if (slot.term.form == query_term_form::variable) {
            add_bound(state, slot.term.variable);
        }
        if (slot.term.form == query_term_form::param) {
            add_use(state,
                param_use{
                    .name = slot.term.param,
                    .shape = param_shape::value,
                    .domain = Slot::cls,
                    .point = false,
                });
        }
    }
    state.items[state.item_count] = body_item{
        .form = body_form::atom,
        .atom = atom,
        .condition = condition_data{},
    };
    ++state.item_count;
}

consteval auto record_condition(rule_state& state, cond_value const& cond)
    -> void {
    if (state.item_count == state.items.size()) {
        rule_has_too_many_conditions();
    }
    state.items[state.item_count] = body_item{
        .form = body_form::condition,
        .atom = atom_data{},
        .condition = cond.data,
    };
    ++state.item_count;
    for (auto index = std::size_t{0}; index != cond.use_count; ++index) {
        add_use(state, cond.uses[index]);
    }
}

// ————————————————————————————————————————————————————————————————————
// Rule assembly: param registry fold, boundness walls, dense variable
// numbering over the written walk, and the head-alignment wall
// (ts/src/query/lower.ts:1337-1426, 1678-1994).
// ————————————————————————————————————————————————————————————————————

struct numberer {
    std::size_t count;
    std::array<coord_ref, max_query_vars> minted;
};

consteval auto var_id(numberer& numbers, coord_ref variable)
    -> std::uint16_t {
    for (auto index = std::size_t{0}; index != numbers.count; ++index) {
        if (numbers.minted[index] == variable) {
            return static_cast<std::uint16_t>(index);
        }
    }
    if (numbers.count == max_query_vars) {
        rule_has_too_many_variables();
    }
    numbers.minted[numbers.count] = variable;
    ++numbers.count;
    return static_cast<std::uint16_t>(numbers.count - 1);
}

consteval auto param_id(query_ir const& ir, name_text name)
    -> std::uint16_t {
    for (auto index = std::size_t{0}; index != ir.param_count; ++index) {
        if (ir.params[index].name == name) {
            return static_cast<std::uint16_t>(index);
        }
    }
    // Unreachable by construction: every param term was recorded with a
    // use, and the uses were folded before numbering.
    query_param_is_inferred_inconsistently_across_uses();
    return 0;
}

consteval auto wire_term_of(query_ir const& ir, numberer& numbers,
    term_data const& term) -> wire_term {
    auto out = wire_term{};
    out.form = term.form;
    switch (term.form) {
    case query_term_form::variable:
    case query_term_form::measure:
        out.var = var_id(numbers, term.variable);
        break;
    case query_term_form::param:
        out.param = param_id(ir, term.param);
        break;
    case query_term_form::literal:
        out.literal = term.literal;
        break;
    case query_term_form::absent:
        break;
    }
    return out;
}

consteval auto fold_uses(query_ir& ir, rule_state const& state) -> void {
    for (auto index = std::size_t{0}; index != state.use_count; ++index) {
        auto const& use = state.uses[index];
        auto found = false;
        for (auto at = std::size_t{0}; at != ir.param_count; ++at) {
            if (!(ir.params[at].name == use.name)) {
                continue;
            }
            found = true;
            if (ir.params[at].shape != use.shape) {
                query_param_is_used_at_two_shapes();
            }
            if (!(ir.params[at].domain == use.domain)) {
                query_param_is_inferred_inconsistently_across_uses();
            }
        }
        if (found) {
            continue;
        }
        if (ir.param_count == max_query_params) {
            query_has_too_many_params();
        }
        ir.params[ir.param_count] = param_data{
            .name = use.name,
            .shape = use.shape,
            .domain = use.domain,
            .point = use.point,
        };
        ++ir.param_count;
    }
}

consteval auto term_is_bound_var(rule_state const& state,
    term_data const& term) -> bool {
    if (term.form != query_term_form::variable
        && term.form != query_term_form::measure) {
        return true;
    }
    return is_bound(state, term.variable);
}

consteval auto append_rule(query_ir& ir, rule_data const& rule) -> void {
    if (ir.rule_count == max_query_rules) {
        query_has_too_many_rules();
    }
    if (rule.find_count == 0) {
        rule_finds_nothing();
    }

    // 1. Param registry fold (first use mints the dense ParamId; one name
    //    keeps one shape and one anchored domain).
    fold_uses(ir, rule.state);

    // 2. Boundness walls (the TS construction-time walls; the engine's
    //    safety refusal stands behind them).
    for (auto index = std::size_t{0}; index != rule.state.item_count;
        ++index) {
        auto const& item = rule.state.items[index];
        if (item.form != body_form::condition) {
            continue;
        }
        if (!term_is_bound_var(rule.state, item.condition.lhs)
            || !term_is_bound_var(rule.state, item.condition.rhs)) {
            where_condition_variable_is_not_bound_in_this_rule();
        }
    }
    for (auto index = std::size_t{0}; index != rule.find_count; ++index) {
        if (!term_is_bound_var(rule.state, rule.finds[index].over)) {
            find_head_variable_is_not_bound_in_this_rule();
        }
        for (auto other = std::size_t{0}; other != index; ++other) {
            if (rule.finds[other].name == rule.finds[index].name) {
                find_head_names_must_be_distinct();
            }
        }
    }

    // 3. Dense variable numbering over the written walk (body items in
    //    written order, bindings in written order, finds LAST) and the
    //    bucketed wire rule.
    auto numbers = numberer{};
    auto out = wire_rule{};
    for (auto index = std::size_t{0}; index != rule.state.item_count;
        ++index) {
        auto const& item = rule.state.items[index];
        if (item.form == body_form::atom) {
            if (out.atom_count == max_query_atoms) {
                rule_has_too_many_atoms();
            }
            auto atom = wire_atom{};
            atom.relation = item.atom.relation;
            atom.binding_count = item.atom.binding_count;
            for (auto binding = std::size_t{0};
                binding != item.atom.binding_count; ++binding) {
                atom.bindings[binding] = wire_binding{
                    .field = static_cast<std::uint16_t>(
                        item.atom.bindings[binding].field),
                    .term = wire_term_of(
                        ir, numbers, item.atom.bindings[binding].term),
                };
            }
            out.atoms[out.atom_count] = atom;
            ++out.atom_count;
        } else {
            if (out.condition_count == max_query_conditions) {
                rule_has_too_many_conditions();
            }
            out.conditions[out.condition_count] = wire_condition{
                .op = item.condition.op,
                .mask = item.condition.mask,
                .lhs = wire_term_of(ir, numbers, item.condition.lhs),
                .rhs = wire_term_of(ir, numbers, item.condition.rhs),
            };
            ++out.condition_count;
        }
    }
    out.find_count = rule.find_count;
    for (auto index = std::size_t{0}; index != rule.find_count; ++index) {
        auto const& column = rule.finds[index];
        out.finds[index] = wire_find{
            .form = column.form,
            .op = column.op,
            .over = var_id(numbers, column.over.variable),
        };
    }

    // 4. The head-alignment wall: every rule derives the same head
    //    (name, shape, op, and answer class, position for position).
    if (ir.rule_count == 0) {
        ir.head_count = rule.find_count;
        for (auto index = std::size_t{0}; index != rule.find_count;
            ++index) {
            ir.head[index] = rule.finds[index];
        }
    } else {
        if (ir.head_count != rule.find_count) {
            every_rule_of_a_query_must_derive_the_same_head();
        }
        for (auto index = std::size_t{0}; index != rule.find_count;
            ++index) {
            auto const& lead = ir.head[index];
            auto const& column = rule.finds[index];
            if (!(lead.name == column.name) || lead.form != column.form
                || lead.op != column.op
                || !(lead.answer == column.answer)) {
                every_rule_of_a_query_must_derive_the_same_head();
            }
        }
    }

    ir.rules[ir.rule_count] = out;
    ++ir.rule_count;
}

// The comparison constructors' shared anchor machinery.

template<class Side>
consteval auto side_is_term() -> bool {
    return is_qvar_v<Side> || is_param_ref_v<Side>
        || is_measure_ref_v<Side>;
}

} // namespace bdb::detail

export namespace bdb {

// ————————————————————————————————————————————————————————————————————
// Predicates (`.where` vocabulary).
// ————————————————————————————————————————————————————————————————————

/// Point membership — THE one spelling (ts/src/query/atom.ts:425-435):
/// `point_in(t, w)` holds iff `w.start ≤ t < w.end`. The point side is a
/// param, a bound element-typed variable, or an integer literal; the
/// interval side is a bound interval variable. The stored value is
/// interval-LEFT whatever the surface order (lowering.md §4.2).
template<class Point, class IntervalVar>
[[nodiscard]] consteval auto point_in(Point point, IntervalVar)
    -> cond_value {
    static_assert(detail::is_qvar_v<IntervalVar>
            && (IntervalVar::cls.kind == value_kind::interval_u64
                || IntervalVar::cls.kind == value_kind::interval_i64),
        "bumbledb point_in(): the interval side must be an interval-typed "
        "query variable (vars.window)");
    constexpr auto element =
        IntervalVar::cls.kind == value_kind::interval_u64
        ? value_kind::u64
        : value_kind::i64;

    auto out = cond_value{};
    out.data.op = query_cmp::point_in;
    out.data.lhs = detail::var_term<IntervalVar>();
    if constexpr (detail::is_param_ref_v<Point>) {
        out.data.rhs = detail::param_term<Point>();
        out.uses[0] = param_use{
            .name = Point::name,
            .shape = param_shape::value,
            .domain = field_class{element, 0},
            .point = true,
        };
        out.use_count = 1;
    } else if constexpr (detail::is_qvar_v<Point>) {
        static_assert(Point::cls.kind == element,
            detail::kind_mismatch_message<Point::relation_name,
                Point::field_name, Point::cls, IntervalVar::relation_name,
                IntervalVar::field_name,
                field_class{element, 0}>("be the point of"));
        out.data.rhs = detail::var_term<Point>();
    } else {
        static_assert(std::integral<Point>,
            "bumbledb point_in(): the point side is a param, a bound "
            "variable, or an integer literal");
        out.data.rhs = detail::literal_term(
            detail::scalar_literal(element, point));
    }
    return out;
}

/// THE interval-pair comparison (`ir::CmpOp::Allen`), the TS-shaped
/// argument order (`allen(window, ALLEN.intersects, incident)`) spelled
/// `allen_in(window, bdb::allen::intersects, r.param<"incident">())` —
/// satisfied iff the pair's Allen classification is in the 13-bit mask.
/// (`bdb::allen` itself is the mask-constant namespace, so the predicate
/// carries the `_in` suffix, like `point_in`.) Sides are bound interval
/// variables, params (anchored by the variable sibling), or interval
/// literals; at least one side is a variable.
template<class Left, class Right>
[[nodiscard]] consteval auto allen_in(Left left, allen_mask mask,
    Right right) -> cond_value {
    constexpr auto left_is_var = detail::is_qvar_v<Left>;
    constexpr auto right_is_var = detail::is_qvar_v<Right>;
    static_assert(left_is_var || right_is_var,
        "bumbledb allen_in(): at least one side must be a bound interval "
        "variable (the anchor that types the comparison)");

    constexpr auto domain = [] {
        if constexpr (left_is_var) {
            return Left::cls;
        } else {
            return Right::cls;
        }
    }();
    static_assert(domain.kind == value_kind::interval_u64
            || domain.kind == value_kind::interval_i64,
        "bumbledb allen_in(): the variable side must be interval-typed");

    auto const side = [&]<class Side>(Side value) -> term_data {
        if constexpr (detail::is_qvar_v<Side>) {
            static_assert(Side::cls == domain,
                "bumbledb allen_in(): both interval sides must share one "
                "element domain");
            return detail::var_term<Side>();
        } else if constexpr (detail::is_param_ref_v<Side>) {
            return detail::param_term<Side>();
        } else {
            static_assert(
                std::same_as<Side, interval<std::uint64_t>>
                    || std::same_as<Side, interval<std::int64_t>>,
                "bumbledb allen_in(): a literal side must be a "
                "bdb::interval of the variable side's element domain");
            return detail::literal_term(detail::interval_literal(value));
        }
    };

    auto out = cond_value{};
    out.data.op = query_cmp::allen;
    out.data.mask = mask.bits();
    out.data.lhs = side(left);
    out.data.rhs = side(right);
    if constexpr (detail::is_param_ref_v<Left>) {
        out.uses[out.use_count] = param_use{
            .name = Left::name,
            .shape = param_shape::value,
            .domain = domain,
            .point = false,
        };
        ++out.use_count;
    }
    if constexpr (detail::is_param_ref_v<Right>) {
        out.uses[out.use_count] = param_use{
            .name = Right::name,
            .shape = param_shape::value,
            .domain = domain,
            .point = false,
        };
        ++out.use_count;
    }
    return out;
}

} // namespace bdb

namespace bdb::detail {

/// The shared scalar-comparison constructor: sides are bound variables,
/// params (anchored by the variable sibling), measures (order ops), or
/// integral/bool literals (tagged by the sibling's domain).
template<query_cmp Op, class Left, class Right>
consteval auto comparison_of(Left left, Right right) -> cond_value {
    constexpr auto ordered = Op == query_cmp::lt || Op == query_cmp::le
        || Op == query_cmp::gt || Op == query_cmp::ge;
    static_assert(side_is_term<Left>() || side_is_term<Right>(),
        "bumbledb comparison: at least one side must be a bound variable, "
        "a measure, or a param (two literals compare nothing)");
    static_assert(
        (!is_measure_ref_v<Left> && !is_measure_ref_v<Right>) || ordered,
        "bumbledb comparison: a duration/measure side is legal in order "
        "comparisons only (lt/le/gt/ge)");

    // The anchoring domain: a variable side's class, else the measure
    // (u64), else the OTHER side anchors.
    constexpr auto domain = [] {
        if constexpr (is_qvar_v<Left>) {
            return Left::cls;
        } else if constexpr (is_qvar_v<Right>) {
            return Right::cls;
        } else {
            return field_class{value_kind::u64, 0}; // the measure domain
        }
    }();

    if constexpr (is_qvar_v<Left> && is_qvar_v<Right>) {
        static_assert(Left::cls == Right::cls,
            kind_mismatch_message<Left::relation_name, Left::field_name,
                Left::cls, Right::relation_name, Right::field_name,
                Right::cls>("join"));
        static_assert(Left::classed == Right::classed
                && (!Left::classed || Left::law == Right::law),
            cross_class_message<Left::relation_name, Left::field_name,
                Left::classed, Left::law, Right::relation_name,
                Right::field_name, Right::classed, Right::law>("join"));
    }
    if constexpr (ordered) {
        static_assert(domain.kind == value_kind::boolean
                || domain.kind == value_kind::u64
                || domain.kind == value_kind::i64,
            "bumbledb comparison: order comparisons take orderable scalar "
            "sides only (bool/u64/i64/measure) — intervals compare through "
            "bdb::allen and bdb::point_in");
    }

    auto out = cond_value{};
    out.data.op = Op;
    auto const side = [&]<class Side>(Side value) -> term_data {
        if constexpr (is_qvar_v<Side>) {
            return var_term<Side>();
        } else if constexpr (is_measure_ref_v<Side>) {
            return measure_term<typename Side::over>();
        } else if constexpr (is_param_ref_v<Side>) {
            out.uses[out.use_count] = param_use{
                .name = Side::name,
                .shape = param_shape::value,
                .domain = domain,
                .point = false,
            };
            ++out.use_count;
            return param_term<Side>();
        } else if constexpr (std::same_as<Side, bool>) {
            static_assert(domain.kind == value_kind::boolean,
                "bumbledb comparison: a bool literal needs a bool-typed "
                "sibling");
            return literal_term(scalar_literal(domain.kind, value));
        } else {
            static_assert(std::integral<Side>,
                "bumbledb comparison: a literal side must be integral "
                "(strings/bytes/intervals bind through params)");
            static_assert(domain.kind == value_kind::u64
                    || domain.kind == value_kind::i64,
                "bumbledb comparison: an integer literal needs a "
                "u64/i64/measure sibling to type it");
            return literal_term(scalar_literal(domain.kind, value));
        }
    };
    out.data.lhs = side(left);
    out.data.rhs = side(right);
    return out;
}

} // namespace bdb::detail

export namespace bdb {

/// Typed equality (`ir::CmpOp::Eq`).
template<class Left, class Right>
[[nodiscard]] consteval auto eq(Left left, Right right) -> cond_value {
    return detail::comparison_of<query_cmp::eq>(left, right);
}

/// Typed disequality (`ir::CmpOp::Ne`).
template<class Left, class Right>
[[nodiscard]] consteval auto ne(Left left, Right right) -> cond_value {
    return detail::comparison_of<query_cmp::ne>(left, right);
}

/// Strict less-than (`ir::CmpOp::Lt`) — orderable scalar sides only.
template<class Left, class Right>
[[nodiscard]] consteval auto lt(Left left, Right right) -> cond_value {
    return detail::comparison_of<query_cmp::lt>(left, right);
}

/// Less-or-equal (`ir::CmpOp::Le`).
template<class Left, class Right>
[[nodiscard]] consteval auto le(Left left, Right right) -> cond_value {
    return detail::comparison_of<query_cmp::le>(left, right);
}

/// Strict greater-than (`ir::CmpOp::Gt`).
template<class Left, class Right>
[[nodiscard]] consteval auto gt(Left left, Right right) -> cond_value {
    return detail::comparison_of<query_cmp::gt>(left, right);
}

/// Greater-or-equal (`ir::CmpOp::Ge`).
template<class Left, class Right>
[[nodiscard]] consteval auto ge(Left left, Right right) -> cond_value {
    return detail::comparison_of<query_cmp::ge>(left, right);
}

// ————————————————————————————————————————————————————————————————————
// The rule chain.
// ————————————————————————————————————————————————————————————————————

/// The chain after at least one `.match`: value state accumulates; the
/// matched facades ride the TYPE (they shape the find pattern).
template<class S, class... Facades>
struct rule_chain {
    rule_state state{};

    /// Joins another relation into the rule (shared variables ARE the
    /// join — reuse a `vars` member across patterns).
    template<class Facade>
    [[nodiscard]] consteval auto match(Facade,
        match_pattern_of<S, Facade> const& pattern) const
        -> rule_chain<S, Facades..., Facade> {
        static_assert(detail::is_query_facade<Facade>(),
            "bumbledb match(): the first argument must be a relation "
            "facade (bdb::relation<...>)");
        static_assert(detail::facade_in_schema<S, Facade>(),
            detail::foreign_relation_message<S, Facade>());
        auto next = rule_chain<S, Facades..., Facade>{.state = state};
        detail::record_match<S, Facade>(next.state, pattern);
        return next;
    }

    /// Conjoins conditions (each a predicate value — bdb::point_in,
    /// bdb::allen, bdb::eq/ne/lt/le/gt/ge).
    template<class... Conds>
    [[nodiscard]] consteval auto where(Conds const&... conds) const
        -> rule_chain {
        static_assert(
            (std::same_as<std::remove_cvref_t<Conds>, cond_value> && ...),
            "bumbledb where(): every argument must be a predicate value "
            "(bdb::point_in / bdb::allen / bdb::eq / ...)");
        auto next = *this;
        (detail::record_condition(next.state, conds), ...);
        return next;
    }

    /// Ends the rule with its answer head: a designated-init pattern over
    /// the matched relations' coordinates (bound variables only), plus
    /// optional named aggregate columns appended AFTER the pattern
    /// columns (`r.sum<"downtime">(r.duration(vars.window))`). Head order
    /// = pattern coordinate order, then the aggregates in written order.
    template<class... Folds>
    [[nodiscard]] consteval auto find(
        find_pattern_of<S, Facades...> const& head,
        Folds const&... folds) const -> rule_data {
        auto out = rule_data{.state = state, .find_count = 0, .finds = {}};

        using Pattern = find_pattern_of<S, Facades...>;
        constexpr auto members = std::define_static_array(
            std::meta::nonstatic_data_members_of(
                ^^Pattern, std::meta::access_context::current()));
        template for (
            constexpr auto index : detail::index_array<members.size()>()) {
            using Slot = [:std::meta::type_of(members[index]):];
            auto const& slot = head.[:members[index]:];
            if (slot.term.form != query_term_form::absent) {
                if (out.find_count == max_query_finds) {
                    detail::rule_has_too_many_finds();
                }
                out.finds[out.find_count] = find_data{
                    .name = Slot::field_name,
                    .form = find_form::variable,
                    .op = fold_form::sum,
                    .over = slot.term,
                    .answer = Slot::cls,
                };
                ++out.find_count;
            }
        }

        [[maybe_unused]] auto const add_fold = [&]<class Fold>(Fold const&) {
            if (out.find_count == max_query_finds) {
                detail::rule_has_too_many_finds();
            }
            out.finds[out.find_count] = find_data{
                .name = Fold::column_name,
                .form = find_form::aggregate_measure,
                .op = Fold::op,
                .over = detail::var_term<typename Fold::over>(),
                .answer = field_class{value_kind::u64, 0},
            };
            ++out.find_count;
        };
        (add_fold(folds), ...);
        return out;
    }
};

// ————————————————————————————————————————————————————————————————————
// Free mints. The scope members below carry the same vocabulary, but a
// member spelled with EXPLICIT template arguments (`r.param<"t">()`)
// is a dependent template-name inside the generic rule lambda — the
// grammar demands `r.template param<"t">()` — so the ergonomic spelling
// of the name-carrying mints is the free one: `bdb::param<"t">()`,
// `bdb::sum<"downtime">(r.duration(vars.window))`.
// ————————————————————————————————————————————————————————————————————

/// A named scalar parameter; type/point-domain inferred from use
/// (TODO_CPP §21).
template<fixed_string Name>
[[nodiscard]] consteval auto param() -> param_ref<Name> {
    return {};
}

/// A named sum-of-measure head column:
/// `bdb::sum<"downtime">(r.duration(vars.window))`.
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto sum(measure_ref<Var>)
    -> fold_ref<Name, Var, fold_form::sum> {
    return {};
}

// ————————————————————————————————————————————————————————————————————
// The rule scope (`auto r`).
// ————————————————————————————————————————————————————————————————————

/// What the rule lambda receives: variable mints, params, measures,
/// aggregates, and the chain starter.
template<class S>
struct rule_scope {
    /// Mints the relation's variable product — one member per field,
    /// named per field, member access only (module comment).
    template<class Facade>
    [[nodiscard]] consteval auto vars(Facade) const -> vars_of<S, Facade> {
        static_assert(detail::is_query_facade<Facade>(),
            "bumbledb r.vars(): the argument must be a relation facade "
            "(bdb::relation<...>)");
        static_assert(detail::facade_in_schema<S, Facade>(),
            detail::foreign_relation_message<S, Facade>());
        return {};
    }

    /// The member twin of `bdb::param` (spell it `r.template param<"t">()`
    /// — the grammar's price for a dependent template-name).
    template<fixed_string Name>
    [[nodiscard]] consteval auto param() const -> param_ref<Name> {
        return {};
    }

    /// The measure of an interval variable (u64 point count).
    template<class Var>
    [[nodiscard]] consteval auto duration(Var) const -> measure_ref<Var> {
        static_assert(detail::is_qvar_v<Var>
                && (Var::cls.kind == value_kind::interval_u64
                    || Var::cls.kind == value_kind::interval_i64),
            "bumbledb r.duration(): the argument must be an interval-typed "
            "query variable — a duration is an interval's measure");
        return {};
    }

    /// The member twin of `bdb::sum` (spell it `r.template sum<...>`).
    template<fixed_string Name, class Var>
    [[nodiscard]] consteval auto sum(measure_ref<Var>) const
        -> fold_ref<Name, Var, fold_form::sum> {
        return {};
    }

    /// Starts the rule body with one positive EDB atom.
    template<class Facade>
    [[nodiscard]] consteval auto match(Facade facade,
        match_pattern_of<S, Facade> const& pattern) const
        -> rule_chain<S, Facade> {
        return rule_chain<S>{}.match(facade, pattern);
    }
};

// ————————————————————————————————————————————————————————————————————
// The query value.
// ————————————————————————————————————————————————————————————————————

/// A whole query as one structural literal: the lowered IR rides the
/// value (NTTP-friendly — `db.prepare<DownAt>()`), the schema ties the
/// TYPE. `.rule` appends one rule; every rule must derive the same head.
template<class S>
struct query_value {
    query_ir ir{};

    template<class Build>
    [[nodiscard]] consteval auto rule(Build build) const -> query_value {
        auto const result = build(rule_scope<S>{});
        static_assert(
            std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>,
            "bumbledb query.rule(): the rule body must end in .find(...)");
        auto next = *this;
        if constexpr (std::same_as<std::remove_cvref_t<decltype(result)>,
                          rule_data>) {
            detail::append_rule(next.ir, result);
        }
        return next;
    }
};

/// The query entry point: `bdb::query(Uptime).rule([](auto r) consteval
/// {...})` (TODO_CPP §11). The schema VALUE selects the TYPE; the type
/// carries everything the elaboration needs (tables and class map are
/// type-derivable — schema_value::member_relation_table).
template<Theory S>
[[nodiscard]] consteval auto query(S const&) -> query_value<S> {
    return {};
}

} // namespace bdb

namespace bdb::detail {

// ————————————————————————————————————————————————————————————————————
// Product synthesis off the QUERY VALUE (row + params products).
// ————————————————————————————————————————————————————————————————————

// `^^std::uint64_t` is ill-formed on the pinned GCC ("'^^' cannot be
// applied to a using-declaration"); routing through a template parameter
// resolves the alias during substitution first.
template<class T>
inline constexpr auto query_type_reflection = ^^T;

consteval auto answer_type_of(field_class cls) -> std::meta::info {
    switch (cls.kind) {
    case value_kind::boolean:
        return query_type_reflection<bool>;
    case value_kind::u64:
        return query_type_reflection<std::uint64_t>;
    case value_kind::i64:
        return query_type_reflection<std::int64_t>;
    case value_kind::string:
        // Borrowed from the answers carrier (TODO_CPP §22).
        return query_type_reflection<std::string_view>;
    case value_kind::fixed_bytes:
        // Borrowed from the answers carrier (TODO_CPP §22).
        return query_type_reflection<std::span<std::byte const>>;
    case value_kind::interval_u64:
        return query_type_reflection<interval<std::uint64_t>>;
    case value_kind::interval_i64:
        break;
    }
    return query_type_reflection<interval<std::int64_t>>;
}

consteval auto param_type_of(param_data const& parameter)
    -> std::meta::info {
    if (parameter.shape == param_shape::mask) {
        return query_type_reflection<allen_mask>;
    }
    // The scalar lane (sets arrive with a later phase): the anchored
    // domain IS the member type — a point-domain param is its element.
    return answer_type_of(parameter.domain);
}

/// The synthesized answer-row product: one member per head column, named
/// per column, typed by the column's answer class (TODO_CPP §12).
template<auto Query>
struct query_row_types {
    struct Row;
    consteval {
        auto specs = std::vector<std::meta::info>{};
        for (auto index = std::size_t{0}; index != Query.ir.head_count;
            ++index) {
            specs.push_back(std::meta::data_member_spec(
                answer_type_of(Query.ir.head[index].answer),
                {.name = spec_name(Query.ir.head[index].name.view())}));
        }
        std::meta::define_aggregate(^^Row, specs);
    }
};

/// The synthesized params product: one member per registered param in
/// registry order (= positional bind order), named per param, typed by
/// the anchored domain. A wrong name or type at `execute` is therefore a
/// compile error (TODO_CPP §21).
template<auto Query>
struct query_params_types {
    struct Params;
    consteval {
        auto specs = std::vector<std::meta::info>{};
        for (auto index = std::size_t{0}; index != Query.ir.param_count;
            ++index) {
            specs.push_back(std::meta::data_member_spec(
                param_type_of(Query.ir.params[index]),
                {.name = spec_name(Query.ir.params[index].name.view())}));
        }
        std::meta::define_aggregate(^^Params, specs);
    }
};

} // namespace bdb::detail

export namespace bdb {

/// The query's synthesized answer-row product (TODO_CPP §12).
template<auto Query>
using row_of = typename detail::query_row_types<Query>::Row;

/// The query's synthesized params product (TODO_CPP §21).
template<auto Query>
using params_of = typename detail::query_params_types<Query>::Params;

} // namespace bdb
