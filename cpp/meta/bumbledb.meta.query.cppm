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

/// Most recursive predicates one program may declare (the engine's own
/// MAX_PREDICATES is 16; lowering.md §4.1).
inline constexpr std::size_t max_program_recs = 4;

/// Most handles one closed-membership array may spell in a match record.
inline constexpr std::size_t max_membership_handles = 8;

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
/// wildcard — an unmentioned field binds nothing. `param_set` is the
/// ∈-set binding: a closed-membership array lowers to it over a synthetic
/// content-addressed registry entry whose set is a PROGRAM CONSTANT (the
/// execute-time params product never carries it — lowering.md §4.2).
enum class query_term_form : std::uint8_t {
    absent,
    variable,
    param,
    param_set,
    literal,
    measure,
};

/// One builder-stage term: variables/measures ride their MINT coordinate
/// (the identity `v(Relation).field` established), params their name.
/// A membership term additionally carries its pre-resolved handle row ids
/// (queries resolve handles HOST-side — lowering.md §7.8).
struct term_data {
    query_term_form form;
    coord_ref variable;
    name_text param;
    query_literal literal;
    std::size_t member_count;
    std::array<std::uint64_t, max_membership_handles> members;
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

/// One named binding of a recursive (IDB) atom — `bdb::bind<"c">(var)`:
/// the target head column BY NAME, the variable (IDB bindings are var
/// terms only — ts/src/query/lower.ts:1759-1783), and the variable's
/// class facts for the head-slot join wall.
struct idb_bind_data {
    name_text column;
    coord_ref variable;
    field_class cls;
    bool classed;
    coord_ref law;
};

/// One recursive atom as recorded: the rec's NAME (resolved to its dense
/// PredId at program assembly), polarity, and the named bindings. The
/// binds are placed and numbered in the target's HEAD order at assembly
/// (`FieldId(i)` = head position i — lowering.md §4.2).
struct idb_atom_data {
    name_text pred;
    bool negated;
    std::size_t bind_count;
    std::array<idb_bind_data, max_query_finds> binds;
};

/// One rule-body item: the written interleave of match/where is preserved
/// so variable numbering walks body items in WRITTEN order (lowering.md
/// §4.2), whatever bucket each item later lowers into.
enum class body_form : std::uint8_t {
    atom,
    negated_atom,
    idb_atom,
    condition,
};

struct body_item {
    body_form form;
    atom_data atom;
    idb_atom_data idb;
    condition_data condition;
};

/// A find column's form (`ir::FindTerm`): a projected variable, a
/// var-scoped aggregate (`sum(minor)`, `count()`, `pack(span)`, ...), or
/// an aggregate over the measure (`sum(duration(w))`).
enum class find_form : std::uint8_t {
    variable,
    aggregate,
    aggregate_measure,
};

/// The aggregate ops the heads mint (`ir::AggOp`, all eight).
enum class fold_form : std::uint8_t {
    sum,
    min,
    max,
    count,
    count_distinct,
    arg_max,
    arg_min,
    pack,
};

/// One find column: the answer column name, the term shape, the answer
/// cell's structural class (the row-product synthesis input), the Arg key
/// (`arg_max`/`arg_min` only), and — variable finds — the column's law
/// class (the IDB head-slot join wall's data).
struct find_data {
    name_text name;
    find_form form;
    fold_form op;
    term_data over;
    field_class answer;
    bool has_over;
    bool key_present;
    term_data key;
    bool classed;
    coord_ref law;
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
/// point_in — TODO_CPP §21). A MEMBERSHIP entry is a synthetic
/// content-addressed set param pre-resolved at build: it never appears in
/// the params product, and execution supplies its frozen set positionally
/// from the query constant (ts/src/query/run.ts:57-63).
struct param_data {
    name_text name;
    param_shape shape;
    field_class domain;
    bool point;
    bool membership;
    std::size_t member_count;
    std::array<std::uint64_t, max_membership_handles> members;
};

/// One param USE, recorded at the position that anchors it.
struct param_use {
    name_text name;
    param_shape shape;
    field_class domain;
    bool point;
    bool membership;
    std::size_t member_count;
    std::array<std::uint64_t, max_membership_handles> members;
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

/// One numbered atom: an EDB atom (`idb == false`, `relation` read) or a
/// recursive IDB atom (`idb == true`, `pred` read — the target's dense
/// PredId; bindings address head positions).
struct wire_atom {
    std::uint32_t relation;
    bool idb;
    std::uint16_t pred;
    std::size_t binding_count;
    std::array<wire_binding, max_relation_fields> bindings;
};

struct wire_condition {
    query_cmp op;
    std::uint16_t mask;
    wire_term lhs;
    wire_term rhs;
};

/// One numbered find term. `over` is read for variable/measure columns
/// and for aggregates with `has_over` (nullary `count` has none); the key
/// fields for `arg_max`/`arg_min`.
struct wire_find {
    find_form form;
    fold_form op;
    std::uint16_t over;
    bool has_over;
    bool key_present;
    bool key_is_measure;
    std::uint16_t key;
};

/// One numbered rule, bucketed exactly as the bridge's `bdb_rule` reads
/// it (positive atoms / negated atoms / conditions, each in written
/// order).
struct wire_rule {
    std::size_t atom_count;
    std::array<wire_atom, max_query_atoms> atoms;
    std::size_t negated_count;
    std::array<wire_atom, max_query_atoms> negated;
    std::size_t condition_count;
    std::array<wire_condition, max_query_conditions> conditions;
    std::size_t find_count;
    std::array<wire_find, max_query_finds> finds;
};

/// One lowered recursive predicate: its NAME (idb resolution + walls),
/// its head (rule 0's finds — the sealed signature), and its numbered
/// rules.
struct pred_ir {
    name_text head_name;
    std::size_t rule_count;
    std::array<wire_rule, max_query_rules> rules;
    std::size_t head_count;
    std::array<find_data, max_query_finds> head;
};

/// The whole lowered query/program: the recs in declaration order
/// (`PredId` = index), the OUTPUT predicate's rules/head (a plain query is
/// the degenerate no-rec program, `output = 0` — lowering.md §4.1), plus
/// the head columns (row-product synthesis) and the param registry
/// (params-product synthesis, recs' uses folded FIRST — §4.2).
struct query_ir {
    std::size_t rule_count;
    std::array<wire_rule, max_query_rules> rules;
    std::size_t head_count;
    std::array<find_data, max_query_finds> head;
    std::size_t param_count;
    std::array<param_data, max_query_params> params;
    std::size_t rec_count;
    std::array<pred_ir, max_program_recs> recs;
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

/// A set-valued query parameter (`ir::Term::ParamSet`, TODO_CPP §21) —
/// `bdb::set_param<"frontier">()`: bound at execution to a SEQUENCE of
/// values of the anchoring field's type (the params-product member is a
/// span); a binding position matches iff the field value is in the set.
/// Legal in atom bindings (positive and negated) and as one side of `eq`
/// — nowhere else, exactly as the IR rules it.
template<fixed_string Name>
struct set_param_ref {
    static constexpr name_text name = detail::to_name_text(Name.view());
};

/// The measure of an interval variable — `r.duration(vars.window)`:
/// `|[s, e)| = e − s`, u64 (`ir::Term::Measure`).
template<class Var>
struct measure_ref {
    using over = Var;
};

/// One named aggregate head column — `bdb::sum<"downtime">(r.duration(w))`,
/// `bdb::sum<"total">(vars.minor)`, `bdb::count<"n">()`,
/// `bdb::pack<"free">(vars.span)`, `bdb::arg_max<"top">(v, key)`.
/// The NAME is carried here because a C++ designated-init head cannot
/// mint new member names the way a TS object literal can; the aggregate
/// column's name is the one datum the pattern product cannot express.
/// `Over` is a qvar, a measure_ref, or void (nullary count); `Key` a qvar
/// or measure_ref on the Arg ops, void elsewhere.
template<fixed_string Name, fold_form Op, class Over, class Key>
struct agg_ref {
    using over = Over;
    using key = Key;
    static constexpr name_text column_name = detail::to_name_text(Name.view());
    static constexpr fold_form op = Op;
};

/// One NAMED variable head column — `bdb::as<"c">(vars.id)`: the head
/// column name decoupled from the field name (recursive predicates whose
/// rules match different relations need it; the TS object-literal head
/// names freely). Passed to `.find` among the trailing columns.
template<fixed_string Name, class Var>
struct named_find {
    using var = Var;
    static constexpr name_text column_name = detail::to_name_text(Name.view());
};

/// A recursive predicate's reference tag — `bdb::pred<"reach">` names the
/// rec an `.idb`/`.not_idb` atom targets (resolution to the dense PredId
/// happens at program assembly).
template<fixed_string Name>
struct pred_tag {
    static constexpr name_text name = detail::to_name_text(Name.view());
};

template<fixed_string Name>
inline constexpr auto pred = pred_tag<Name>{};

/// One named binding of a recursive atom — `bdb::bind<"c">(vars.parent)`:
/// the target head column BY NAME, the bound variable as the value.
template<fixed_string Column, class Var>
struct idb_bind {
    using var = Var;
    static constexpr name_text column = detail::to_name_text(Column.view());
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
inline constexpr bool is_set_param_ref_v = false;

template<fixed_string Name>
inline constexpr bool is_set_param_ref_v<set_param_ref<Name>> = true;

template<class T>
inline constexpr bool is_measure_ref_v = false;

template<class Var>
inline constexpr bool is_measure_ref_v<measure_ref<Var>> = true;

template<class T>
inline constexpr bool is_agg_ref_v = false;

template<fixed_string Name, fold_form Op, class Over, class Key>
inline constexpr bool is_agg_ref_v<agg_ref<Name, Op, Over, Key>> = true;

template<class T>
inline constexpr bool is_named_find_v = false;

template<fixed_string Name, class Var>
inline constexpr bool is_named_find_v<named_find<Name, Var>> = true;

template<class T>
inline constexpr bool is_idb_bind_v = false;

template<fixed_string Column, class Var>
inline constexpr bool is_idb_bind_v<idb_bind<Column, Var>> = true;

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

/// A queryable schema member: an ordinary all-coordinate facade or a
/// closed relation facade (TODO_CPP §8: closed relations stay query
/// atoms).
template<class Facade>
consteval auto is_query_member() -> bool {
    return is_query_facade<Facade>() || is_closed_facade<Facade>();
}

/// One facade member's column facts, uniform over both member kinds:
/// `include == false` on a closed facade's non-column members (handle
/// constants, the axiom readback, the wire carrier).
struct member_facts {
    bool include;
    std::meta::info value_type;
    name_text relation;
    name_text field;
    std::size_t ordinal;
    field_class cls;
};

consteval auto facts_of_member(std::meta::info member) -> member_facts {
    auto const t = std::meta::dealias(std::meta::type_of(member));
    if (!std::meta::is_class_type(t)
        || !std::meta::has_template_arguments(t)) {
        return member_facts{.include = false,
            .value_type = std::meta::info{},
            .relation = {},
            .field = {},
            .ordinal = 0,
            .cls = {}};
    }
    auto const tmpl = std::meta::template_of(t);
    if (tmpl == ^^coord) {
        auto const args = std::meta::template_arguments_of(t);
        return member_facts{.include = true,
            .value_type = args[0],
            .relation = std::meta::extract<name_text>(args[1]),
            .field = std::meta::extract<name_text>(args[2]),
            .ordinal = std::meta::extract<std::size_t>(args[3]),
            .cls = std::meta::extract<field_class>(args[4])};
    }
    if (tmpl == ^^closed_id) {
        auto const args = std::meta::template_arguments_of(t);
        return member_facts{.include = true,
            .value_type = std::meta::substitute(^^closed_ref, {args[0]}),
            .relation = std::meta::extract<name_text>(args[0]),
            .field = to_name_text("id"),
            .ordinal = 0,
            .cls = field_class{value_kind::u64, 0}};
    }
    return member_facts{.include = false,
        .value_type = std::meta::info{},
        .relation = {},
        .field = {},
        .ordinal = 0,
        .cls = {}};
}

template<class Facade>
consteval auto facade_relation_name() -> name_text {
    auto const members = std::meta::nonstatic_data_members_of(
        ^^Facade, std::meta::access_context::current());
    return facts_of_member(members[0]).relation;
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

/// The §34 wrong-vocabulary wall at a match binding: names the handle,
/// its vocabulary, the coordinate, and the coordinate's vocabulary.
template<name_text HandleRoster, name_text Handle, name_text AtRel,
    name_text AtField, name_text FieldRoster>
consteval auto handle_binding_message() -> std::string {
    return "bumbledb query: handle \"" + std::string{Handle.view()}
        + "\" of closed relation \"" + std::string{HandleRoster.view()}
        + "\" cannot bind coordinate \""
        + coordinate_label(AtRel, AtField)
        + "\" — the field references closed relation \""
        + std::string{FieldRoster.view()} + "\"";
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
auto membership_array_needs_at_least_two_handles() -> void;
auto membership_array_has_duplicate_handles() -> void;
auto membership_array_has_too_many_handles() -> void;
auto find_head_names_must_be_distinct() -> void;
auto every_rule_of_a_query_must_derive_the_same_head() -> void;
auto negated_atom_binds_a_variable_no_positive_atom_binds() -> void;
auto pack_stands_alone_never_beside_another_aggregate() -> void;
auto a_recursive_atom_requires_a_program() -> void;
auto idb_atom_names_no_declared_rec() -> void;
auto a_recursive_rule_matches_only_its_own_rec() -> void;
auto a_recursive_rule_negates_no_stratum() -> void;
auto a_recursive_rule_head_projects_bound_variables_only() -> void;
auto idb_atom_omits_a_head_column() -> void;
auto idb_atom_binds_a_name_the_head_does_not_carry() -> void;
auto idb_binding_joins_only_its_head_columns_class() -> void;
auto program_rec_names_must_be_distinct() -> void;
auto program_rec_defines_at_least_one_rule() -> void;
auto program_has_too_many_recs() -> void;

// ————————————————————————————————————————————————————————————————————
// Term helpers.
// ————————————————————————————————————————————————————————————————————

template<class Var>
consteval auto var_term() -> term_data {
    auto out = term_data{};
    out.form = query_term_form::variable;
    out.variable = coord_ref{
        .relation = Var::relation_name, .field = Var::field_name};
    return out;
}

template<class Var>
consteval auto measure_term() -> term_data {
    auto out = var_term<Var>();
    out.form = query_term_form::measure;
    return out;
}

template<class Param>
consteval auto param_term() -> term_data {
    auto out = term_data{};
    out.form = query_term_form::param;
    out.param = Param::name;
    return out;
}

template<class Param>
consteval auto set_param_term() -> term_data {
    auto out = term_data{};
    out.form = query_term_form::param_set;
    out.param = Param::name;
    return out;
}

consteval auto literal_term(query_literal literal) -> term_data {
    auto out = term_data{};
    out.form = query_term_form::literal;
    out.literal = literal;
    return out;
}

/// The membership registry entry's content-addressed synthetic name:
/// "in <Roster> <sorted row ids>" — one identical array in two positions
/// folds to ONE set param (the TS content-addressed registry,
/// query/lower.ts:453-487). Never a params-product member; the embedded
/// space keeps it disjoint from every user-spellable param name.
consteval auto membership_param_name(name_text roster,
    std::array<std::uint64_t, max_membership_handles> const& members,
    std::size_t count) -> name_text {
    auto text = std::string{"in "} + std::string{roster.view()};
    for (auto index = std::size_t{0}; index != count; ++index) {
        text += index == 0 ? " " : ",";
        text += render_size(members[index]);
    }
    return to_name_text(text);
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

    /// A set-param binding (`{.a = bdb::set_param<"frontier">()}`): the
    /// position matches iff the field value is IN the bound set —
    /// `ir::Term::ParamSet`, anchored at this field's domain (TODO_CPP
    /// §21). Unlike a membership ARRAY, the set arrives at execution.
    template<fixed_string Name>
    consteval binding_slot(set_param_ref<Name> parameter) {
        term = set_param_term<decltype(parameter)>();
    }

    /// A bare literal at a fixed-width field (field-directed tagging).
    consteval binding_slot(T value)
        requires (!is_closed_ref_v<T>
            && (Class.kind == value_kind::boolean
                || Class.kind == value_kind::u64
                || Class.kind == value_kind::i64))
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

    /// A handle literal at a closed-reference field (`{.priority =
    /// Priority.Urgent}`): the HOST resolves it — roster-verified here,
    /// lowered as the declaration-order row id, u64-tagged
    /// (lowering.md §4.2's taggedHandleId; §7.8).
    template<name_text HandleRoster, name_text Handle, std::uint64_t Index>
    consteval binding_slot(handle_value<HandleRoster, Handle, Index>)
        requires is_closed_ref_v<T>
    {
        static_assert(HandleRoster == T::roster_name,
            handle_binding_message<HandleRoster, Handle, Relation, Field,
                T::roster_name>());
        term = literal_term(
            scalar_literal(value_kind::u64, std::uint64_t{Index}));
    }

    /// A membership ARRAY at a closed-reference field (`{.priority =
    /// {Priority.Normal, Priority.Urgent}}`): closed-only in match
    /// records, folded to a pre-resolved ∈-set over a synthetic
    /// content-addressed registry entry (lowering.md §4.2's membership
    /// arrays). Each element's roster wall runs on the closed_ref
    /// conversion.
    consteval binding_slot(std::initializer_list<T> handles)
        requires is_closed_ref_v<T>
    {
        auto members = std::array<std::uint64_t, max_membership_handles>{};
        auto count = std::size_t{0};
        for (auto const& entry : handles) {
            if (count == max_membership_handles) {
                membership_array_has_too_many_handles();
            }
            members[count] = entry.row;
            ++count;
        }
        if (count < 2) {
            membership_array_needs_at_least_two_handles();
        }
        // Sorted row ids are the content address (order-insensitive
        // dedup, mirroring the TS sorted-members registry name).
        for (auto first = std::size_t{1}; first != count; ++first) {
            auto const value = members[first];
            auto at = first;
            while (at != 0 && members[at - 1] > value) {
                members[at] = members[at - 1];
                --at;
            }
            members[at] = value;
        }
        for (auto index = std::size_t{1}; index != count; ++index) {
            if (members[index - 1] == members[index]) {
                membership_array_has_duplicate_handles();
            }
        }
        term.form = query_term_form::param_set;
        term.param =
            membership_param_name(T::roster_name, members, count);
        term.member_count = count;
        term.members = members;
    }
};

/// One find-pattern slot: accepts a class-equal bound variable only (the
/// boundness wall is value-tier, judged at rule assembly).
template<class T, name_text Relation, name_text Field, field_class Class,
    bool Classed, coord_ref Law>
struct find_slot {
    static constexpr name_text field_name = Field;
    static constexpr field_class cls = Class;
    static constexpr bool classed = Classed;
    static constexpr coord_ref law = Law;

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
            auto const facts = facts_of_member(member);
            if (!facts.include) {
                continue;
            }
            auto const law = law_of<S>(facts.relation, facts.field);
            specs.push_back(std::meta::data_member_spec(
                std::meta::substitute(^^qvar,
                    {facts.value_type,
                        std::meta::reflect_constant(facts.relation),
                        std::meta::reflect_constant(facts.field),
                        std::meta::reflect_constant(facts.cls),
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
            auto const facts = facts_of_member(member);
            if (!facts.include) {
                continue;
            }
            auto const law = law_of<S>(facts.relation, facts.field);
            specs.push_back(std::meta::data_member_spec(
                std::meta::substitute(^^binding_slot,
                    {facts.value_type,
                        std::meta::reflect_constant(facts.relation),
                        std::meta::reflect_constant(facts.field),
                        std::meta::reflect_constant(facts.ordinal),
                        std::meta::reflect_constant(facts.cls),
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
                auto const facts = facts_of_member(member);
                if (!facts.include) {
                    continue;
                }
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
                auto const law = law_of<S>(facts.relation, facts.field);
                specs.push_back(std::meta::data_member_spec(
                    std::meta::substitute(^^find_slot,
                        {facts.value_type,
                            std::meta::reflect_constant(facts.relation),
                            std::meta::reflect_constant(facts.field),
                            std::meta::reflect_constant(facts.cls),
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
    match_pattern_of<S, Facade> const& pattern, bool negated) -> void {
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
        if (slot.term.form == query_term_form::variable && !negated) {
            // A negated atom binds nothing — only rejects (the safety
            // rule); its variables must be bound by positive atoms.
            add_bound(state, slot.term.variable);
        }
        if (slot.term.form == query_term_form::param) {
            auto use = param_use{};
            use.name = slot.term.param;
            use.shape = param_shape::value;
            use.domain = Slot::cls;
            add_use(state, use);
        }
        if (slot.term.form == query_term_form::param_set) {
            // A membership ARRAY entry (member_count != 0) is a synthetic
            // pre-resolved set param (never a params-product member;
            // execution supplies the frozen set positionally). A runtime
            // SET param (member_count == 0) IS a params-product member —
            // its sequence arrives at execute.
            auto use = param_use{};
            use.name = slot.term.param;
            use.shape = param_shape::set;
            use.domain = Slot::cls;
            use.membership = slot.term.member_count != 0;
            use.member_count = slot.term.member_count;
            use.members = slot.term.members;
            add_use(state, use);
        }
    }
    state.items[state.item_count] = body_item{
        .form = negated ? body_form::negated_atom : body_form::atom,
        .atom = atom,
        .idb = idb_atom_data{},
        .condition = condition_data{},
    };
    ++state.item_count;
}

/// Records one recursive atom (either polarity). A POSITIVE idb atom
/// binds its variables (grounding — reach's step rule); a negated one
/// binds nothing.
consteval auto record_idb(rule_state& state, idb_atom_data const& atom)
    -> void {
    if (state.item_count == state.items.size()) {
        rule_has_too_many_atoms();
    }
    if (!atom.negated) {
        for (auto index = std::size_t{0}; index != atom.bind_count;
            ++index) {
            add_bound(state, atom.binds[index].variable);
        }
    }
    state.items[state.item_count] = body_item{
        .form = body_form::idb_atom,
        .atom = atom_data{},
        .idb = atom,
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
        .idb = idb_atom_data{},
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
    case query_term_form::param_set:
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
            .membership = use.membership,
            .member_count = use.member_count,
            .members = use.members,
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

inline constexpr std::size_t no_rec = ~std::size_t{0};

/// The rec a recursive atom targets, by name (nowhere = the wall).
consteval auto rec_index_of(query_ir const& ir, name_text name)
    -> std::size_t {
    for (auto index = std::size_t{0}; index != ir.rec_count; ++index) {
        if (ir.recs[index].head_name == name) {
            return index;
        }
    }
    return no_rec;
}

/// Lowers one recursive atom against its target's SEALED head: binds are
/// placed and numbered in HEAD order — `FieldId(i)` = head position i,
/// every head column bound exactly once (lowering.md §4.2) — and each
/// bind's variable must join its head column's class (the TS fieldJoins
/// wall on the idb pairing).
consteval auto wire_idb_of(query_ir const& ir, numberer& numbers,
    idb_atom_data const& atom, std::size_t pred) -> wire_atom {
    auto const& target = ir.recs[pred];
    auto out = wire_atom{};
    out.idb = true;
    out.pred = static_cast<std::uint16_t>(pred);
    for (auto index = std::size_t{0}; index != atom.bind_count; ++index) {
        auto known = false;
        for (auto column = std::size_t{0}; column != target.head_count;
            ++column) {
            if (target.head[column].name == atom.binds[index].column) {
                known = true;
            }
        }
        if (!known) {
            idb_atom_binds_a_name_the_head_does_not_carry();
        }
    }
    for (auto column = std::size_t{0}; column != target.head_count;
        ++column) {
        auto const& head = target.head[column];
        auto bound = no_rec;
        for (auto index = std::size_t{0}; index != atom.bind_count;
            ++index) {
            if (atom.binds[index].column == head.name) {
                bound = index;
            }
        }
        if (bound == no_rec) {
            idb_atom_omits_a_head_column();
        }
        auto const& bind = atom.binds[bound];
        if (!(bind.cls == head.answer) || bind.classed != head.classed
            || (head.classed && !(bind.law == head.law))) {
            idb_binding_joins_only_its_head_columns_class();
        }
        auto term = wire_term{};
        term.form = query_term_form::variable;
        term.var = var_id(numbers, bind.variable);
        out.bindings[out.binding_count] = wire_binding{
            .field = static_cast<std::uint16_t>(column),
            .term = term,
        };
        ++out.binding_count;
    }
    return out;
}

/// Lowers one assembled rule to its numbered wire form. `self` is the
/// owning rec's index for RECURSIVE rules (their idb atoms may target
/// only the rec itself, positively — the self-recursion cut and the
/// monotonicity wall) and `no_rec` for output/query rules. Requires every
/// rec head in `ir.recs` to be sealed and this rule's param uses folded.
consteval auto lower_rule(query_ir const& ir, rule_data const& rule,
    std::size_t self) -> wire_rule {
    if (rule.find_count == 0) {
        rule_finds_nothing();
    }

    // 1. Boundness and polarity walls (the TS construction-time walls;
    //    the engine's safety refusal stands behind them). The bound set
    //    carries POSITIVE bindings only (record_match/record_idb).
    for (auto index = std::size_t{0}; index != rule.state.item_count;
        ++index) {
        auto const& item = rule.state.items[index];
        if (item.form == body_form::condition) {
            if (!term_is_bound_var(rule.state, item.condition.lhs)
                || !term_is_bound_var(rule.state, item.condition.rhs)) {
                where_condition_variable_is_not_bound_in_this_rule();
            }
        }
        if (item.form == body_form::negated_atom) {
            for (auto binding = std::size_t{0};
                binding != item.atom.binding_count; ++binding) {
                if (!term_is_bound_var(
                        rule.state, item.atom.bindings[binding].term)) {
                    negated_atom_binds_a_variable_no_positive_atom_binds();
                }
            }
        }
        if (item.form == body_form::idb_atom) {
            if (self != no_rec) {
                if (item.idb.negated) {
                    a_recursive_rule_negates_no_stratum();
                }
                if (!(item.idb.pred == ir.recs[self].head_name)) {
                    a_recursive_rule_matches_only_its_own_rec();
                }
            }
            if (item.idb.negated) {
                for (auto bind = std::size_t{0};
                    bind != item.idb.bind_count; ++bind) {
                    if (!is_bound(rule.state,
                            item.idb.binds[bind].variable)) {
                        negated_atom_binds_a_variable_no_positive_atom_binds();
                    }
                }
            }
        }
    }
    auto pack_count = std::size_t{0};
    auto fold_count = std::size_t{0};
    for (auto index = std::size_t{0}; index != rule.find_count; ++index) {
        auto const& column = rule.finds[index];
        if (self != no_rec && column.form != find_form::variable) {
            // Aggregation/measure through a cycle is unrepresentable —
            // the strata judge's roster, made unwritable.
            a_recursive_rule_head_projects_bound_variables_only();
        }
        if (column.form != find_form::variable) {
            if (column.op == fold_form::pack) {
                ++pack_count;
            } else {
                ++fold_count;
            }
        }
        if ((column.has_over
                && !term_is_bound_var(rule.state, column.over))
            || (column.key_present
                && !term_is_bound_var(rule.state, column.key))) {
            find_head_variable_is_not_bound_in_this_rule();
        }
        for (auto other = std::size_t{0}; other != index; ++other) {
            if (rule.finds[other].name == rule.finds[index].name) {
                find_head_names_must_be_distinct();
            }
        }
    }
    if (pack_count > 1 || (pack_count == 1 && fold_count != 0)) {
        // At most one pack per find, never beside a fold or an Arg entry
        // (ts/src/query/find.ts).
        pack_stands_alone_never_beside_another_aggregate();
    }

    // 2. Dense variable numbering over the written walk (body items in
    //    written order, bindings in written order — idb binds in HEAD
    //    order — finds LAST) and the bucketed wire rule.
    auto numbers = numberer{};
    auto out = wire_rule{};
    for (auto index = std::size_t{0}; index != rule.state.item_count;
        ++index) {
        auto const& item = rule.state.items[index];
        if (item.form == body_form::atom
            || item.form == body_form::negated_atom) {
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
            if (item.form == body_form::atom) {
                if (out.atom_count == max_query_atoms) {
                    rule_has_too_many_atoms();
                }
                out.atoms[out.atom_count] = atom;
                ++out.atom_count;
            } else {
                if (out.negated_count == max_query_atoms) {
                    rule_has_too_many_atoms();
                }
                out.negated[out.negated_count] = atom;
                ++out.negated_count;
            }
        } else if (item.form == body_form::idb_atom) {
            auto const pred = rec_index_of(ir, item.idb.pred);
            if (pred == no_rec) {
                if (ir.rec_count == 0) {
                    a_recursive_atom_requires_a_program();
                }
                idb_atom_names_no_declared_rec();
            }
            auto const atom = wire_idb_of(ir, numbers, item.idb, pred);
            if (item.idb.negated) {
                if (out.negated_count == max_query_atoms) {
                    rule_has_too_many_atoms();
                }
                out.negated[out.negated_count] = atom;
                ++out.negated_count;
            } else {
                if (out.atom_count == max_query_atoms) {
                    rule_has_too_many_atoms();
                }
                out.atoms[out.atom_count] = atom;
                ++out.atom_count;
            }
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
        auto find = wire_find{};
        find.form = column.form;
        find.op = column.op;
        find.has_over = column.has_over;
        if (column.key_present) {
            // The Arg key numbers BEFORE the carried value (the TS object
            // literal's evaluation order — lowering.md §4.2 parity).
            find.key_present = true;
            find.key_is_measure =
                column.key.form == query_term_form::measure;
            find.key = var_id(numbers, column.key.variable);
        }
        if (column.has_over) {
            find.over = var_id(numbers, column.over.variable);
        }
        out.finds[index] = find;
    }
    return out;
}

/// The head-alignment wall shared by every predicate: rule 0 seals the
/// head; every later rule derives the same (name, shape, op, answer
/// class, law class), position for position.
consteval auto align_head(std::size_t head_count,
    std::array<find_data, max_query_finds> const& head,
    rule_data const& rule) -> void {
    if (head_count != rule.find_count) {
        every_rule_of_a_query_must_derive_the_same_head();
    }
    for (auto index = std::size_t{0}; index != rule.find_count; ++index) {
        auto const& lead = head[index];
        auto const& column = rule.finds[index];
        if (!(lead.name == column.name) || lead.form != column.form
            || lead.op != column.op || !(lead.answer == column.answer)
            || lead.classed != column.classed
            || (lead.classed && !(lead.law == column.law))) {
            every_rule_of_a_query_must_derive_the_same_head();
        }
    }
}

/// The plain-query append (one OUTPUT rule; no recs in scope).
consteval auto append_rule(query_ir& ir, rule_data const& rule) -> void {
    if (ir.rule_count == max_query_rules) {
        query_has_too_many_rules();
    }
    if (rule.find_count == 0) {
        rule_finds_nothing();
    }

    // Param registry fold (first use mints the dense ParamId; one name
    // keeps one shape and one anchored domain), then the numbered rule.
    fold_uses(ir, rule.state);
    auto const out = lower_rule(ir, rule, no_rec);

    if (ir.rule_count == 0) {
        ir.head_count = rule.find_count;
        for (auto index = std::size_t{0}; index != rule.find_count;
            ++index) {
            ir.head[index] = rule.finds[index];
        }
    } else {
        align_head(ir.head_count, ir.head, rule);
    }

    ir.rules[ir.rule_count] = out;
    ++ir.rule_count;
}

// The comparison constructors' shared anchor machinery.

template<class Side>
consteval auto side_is_term() -> bool {
    return is_qvar_v<Side> || is_param_ref_v<Side>
        || is_set_param_ref_v<Side> || is_measure_ref_v<Side>;
}

/// One named aggregate's find_data (the type walls ran on the mint).
template<class Fold>
consteval auto fold_find_of() -> find_data {
    auto out = find_data{};
    out.name = Fold::column_name;
    out.op = Fold::op;
    using Over = typename Fold::over;
    using Key = typename Fold::key;
    if constexpr (std::same_as<Over, void>) {
        // Nullary count: |the group's distinct full bindings|, u64.
        out.form = find_form::aggregate;
        out.has_over = false;
        out.answer = field_class{value_kind::u64, 0};
    } else if constexpr (is_measure_ref_v<Over>) {
        out.form = find_form::aggregate_measure;
        out.has_over = true;
        out.over = var_term<typename Over::over>();
        out.answer = field_class{value_kind::u64, 0};
    } else {
        out.form = find_form::aggregate;
        out.has_over = true;
        out.over = var_term<Over>();
        // Folds carry their input's type; countDistinct is a cardinality
        // (u64); pack carries the interval type (ts/src/query/find.ts).
        if constexpr (Fold::op == fold_form::count_distinct) {
            out.answer = field_class{value_kind::u64, 0};
        } else {
            out.answer = Over::cls;
        }
    }
    if constexpr (!std::same_as<Key, void>) {
        // The Arg key (numbered BEFORE the carried value at lowering).
        out.key_present = true;
        if constexpr (is_measure_ref_v<Key>) {
            out.key = measure_term<typename Key::over>();
        } else {
            out.key = var_term<Key>();
        }
    }
    return out;
}

/// A numeric (sum-able) variable: u64/i64 — bool stays refused.
template<class Var>
consteval auto is_numeric_var() -> bool {
    return is_qvar_v<Var>
        && (Var::cls.kind == value_kind::u64
            || Var::cls.kind == value_kind::i64);
}

/// An orderable variable: bool folds under min/max (false < true).
template<class Var>
consteval auto is_orderable_var() -> bool {
    return is_qvar_v<Var>
        && (Var::cls.kind == value_kind::boolean
            || Var::cls.kind == value_kind::u64
            || Var::cls.kind == value_kind::i64);
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
        auto use = param_use{};
        use.name = Point::name;
        use.shape = param_shape::value;
        use.domain = field_class{element, 0};
        use.point = true;
        out.uses[0] = use;
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
            // Element-domain equality only: mixed WIDTHS compare freely
            // (the r29 rule — widths type storage, not comparison).
            static_assert(Side::cls.kind == domain.kind,
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
        auto use = param_use{};
        use.name = Left::name;
        use.shape = param_shape::value;
        use.domain = domain;
        out.uses[out.use_count] = use;
        ++out.use_count;
    }
    if constexpr (detail::is_param_ref_v<Right>) {
        auto use = param_use{};
        use.name = Right::name;
        use.shape = param_shape::value;
        use.domain = domain;
        out.uses[out.use_count] = use;
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
    static_assert(
        (!is_set_param_ref_v<Left> && !is_set_param_ref_v<Right>)
            || Op == query_cmp::eq,
        "bumbledb comparison: a set param is legal in atom bindings and "
        "one side of eq only (ir::Term::ParamSet)");

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
            auto use = param_use{};
            use.name = Side::name;
            use.shape = param_shape::value;
            use.domain = domain;
            out.uses[out.use_count] = use;
            ++out.use_count;
            return param_term<Side>();
        } else if constexpr (is_set_param_ref_v<Side>) {
            auto use = param_use{};
            use.name = Side::name;
            use.shape = param_shape::set;
            use.domain = domain;
            out.uses[out.use_count] = use;
            ++out.use_count;
            return set_param_term<Side>();
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
        static_assert(detail::is_query_member<Facade>(),
            "bumbledb match(): the first argument must be a relation "
            "facade (bdb::relation<...> or bdb::closed<...>)");
        static_assert(detail::facade_in_schema<S, Facade>(),
            detail::foreign_relation_message<S, Facade>());
        auto next = rule_chain<S, Facades..., Facade>{.state = state};
        detail::record_match<S, Facade>(next.state, pattern, false);
        return next;
    }

    /// One NEGATED EDB atom (the anti-join — `ir::Rule::negated`): the
    /// rule keeps every binding NO matching fact extends. A negated atom
    /// binds nothing — its variables must be bound by positive atoms (the
    /// safety rule; judged at rule assembly). The matched facade does NOT
    /// join the find pattern (nothing of it is bound).
    template<class Facade>
    [[nodiscard]] consteval auto not_match(Facade,
        match_pattern_of<S, Facade> const& pattern) const -> rule_chain {
        static_assert(detail::is_query_member<Facade>(),
            "bumbledb not_match(): the first argument must be a relation "
            "facade (bdb::relation<...> or bdb::closed<...>)");
        static_assert(detail::facade_in_schema<S, Facade>(),
            detail::foreign_relation_message<S, Facade>());
        auto next = *this;
        detail::record_match<S, Facade>(next.state, pattern, true);
        return next;
    }

    /// One POSITIVE recursive atom — `.idb(bdb::pred<"reach">,
    /// bdb::bind<"c">(vars.parent))`: grounds this rule against the
    /// named predicate's set; binds its variables. Inside a rec's own
    /// rules only the rec itself may be named (the self-recursion cut);
    /// output rules join any FINISHED stratum. Every head column of the
    /// target must be bound exactly once (judged at program assembly).
    template<fixed_string Name, class... Binds>
    [[nodiscard]] consteval auto idb(pred_tag<Name>, Binds... binds) const
        -> rule_chain {
        return with_idb<Name, false>(binds...);
    }

    /// The NEGATED finished-stratum atom — `.not_idb(bdb::pred<"seeded">,
    /// bdb::bind<"c">(vars.id))`: rejects every binding the finished
    /// stratum extends (output rules only; a recursive rule negates no
    /// stratum — monotonicity). Binds nothing.
    template<fixed_string Name, class... Binds>
    [[nodiscard]] consteval auto not_idb(pred_tag<Name>, Binds... binds)
        const -> rule_chain {
        return with_idb<Name, true>(binds...);
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
    /// optional trailing columns in written order — named variable
    /// columns (`bdb::as<"c">(vars.id)`) and named aggregates
    /// (`bdb::sum<"downtime">(r.duration(vars.window))`,
    /// `bdb::sum<"total">(vars.minor)`, `bdb::count<"n">()`,
    /// `bdb::pack<"free">(vars.span)`, ...). Head order = pattern
    /// coordinate order, then the trailing columns in written order.
    template<class... Extras>
    [[nodiscard]] consteval auto find(
        find_pattern_of<S, Facades...> const& head,
        Extras const&... extras) const -> rule_data {
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
                    .has_over = true,
                    .key_present = false,
                    .key = {},
                    .classed = Slot::classed,
                    .law = Slot::law,
                };
                ++out.find_count;
            }
        }

        [[maybe_unused]] auto const add_extra =
            [&]<class Extra>(Extra const&) {
            static_assert(detail::is_agg_ref_v<Extra>
                    || detail::is_named_find_v<Extra>,
                "bumbledb find(): every trailing argument must be a named "
                "head column — bdb::as<\"c\">(var) or a named aggregate "
                "(bdb::sum / min / max / count / count_distinct / arg_max "
                "/ arg_min / pack)");
            if (out.find_count == max_query_finds) {
                detail::rule_has_too_many_finds();
            }
            if constexpr (detail::is_named_find_v<Extra>) {
                using Var = typename Extra::var;
                out.finds[out.find_count] = find_data{
                    .name = Extra::column_name,
                    .form = find_form::variable,
                    .op = fold_form::sum,
                    .over = detail::var_term<Var>(),
                    .answer = Var::cls,
                    .has_over = true,
                    .key_present = false,
                    .key = {},
                    .classed = Var::classed,
                    .law = Var::law,
                };
            } else {
                out.finds[out.find_count] =
                    detail::fold_find_of<Extra>();
            }
            ++out.find_count;
        };
        (add_extra(extras), ...);
        return out;
    }

private:
    template<fixed_string Name, bool Negated, class... Binds>
    [[nodiscard]] consteval auto with_idb(Binds...) const -> rule_chain {
        static_assert((detail::is_idb_bind_v<Binds> && ...),
            "bumbledb idb(): every binding must be spelled "
            "bdb::bind<\"column\">(variable)");
        static_assert(sizeof...(Binds) <= max_query_finds,
            "bumbledb idb(): the bindings exceed the head width");
        auto atom = idb_atom_data{};
        atom.pred = detail::to_name_text(Name.view());
        atom.negated = Negated;
        [[maybe_unused]] auto const add = [&]<class Bind>() {
            using Var = typename Bind::var;
            static_assert(detail::is_qvar_v<Var>,
                "bumbledb bind(): the bound value must be a query "
                "variable (vars.field) — idb bindings are variable terms "
                "only");
            atom.binds[atom.bind_count] = idb_bind_data{
                .column = Bind::column,
                .variable = coord_ref{.relation = Var::relation_name,
                    .field = Var::field_name},
                .cls = Var::cls,
                .classed = Var::classed,
                .law = Var::law,
            };
            ++atom.bind_count;
        };
        (add.template operator()<Binds>(), ...);
        auto next = *this;
        detail::record_idb(next.state, atom);
        return next;
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

/// A named SET parameter (`ir::Term::ParamSet`, TODO_CPP §21): bound at
/// execution to a sequence of the anchoring field's element type.
template<fixed_string Name>
[[nodiscard]] consteval auto set_param() -> set_param_ref<Name> {
    return {};
}

/// A named variable head column — `bdb::as<"c">(vars.id)` (the head name
/// decoupled from the field name; recursive heads need it).
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto as(Var) -> named_find<Name, Var> {
    static_assert(detail::is_qvar_v<Var>,
        "bumbledb as(): the argument must be a query variable "
        "(vars.field)");
    return {};
}

/// A named binding of a recursive atom — `bdb::bind<"c">(vars.parent)`.
template<fixed_string Column, class Var>
[[nodiscard]] consteval auto bind(Var) -> idb_bind<Column, Var> {
    static_assert(detail::is_qvar_v<Var>,
        "bumbledb bind(): the argument must be a query variable "
        "(vars.field) — idb bindings are variable terms only");
    return {};
}

/// A named sum-of-measure head column:
/// `bdb::sum<"downtime">(r.duration(vars.window))`.
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto sum(measure_ref<Var>)
    -> agg_ref<Name, fold_form::sum, measure_ref<Var>, void> {
    return {};
}

/// A named sum over a NUMERIC variable — `bdb::sum<"total">(vars.minor)`.
/// Exact checked sum, wide accumulator; bool stays refused (a truth count
/// is spelled over an explicit 0/1 column).
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto sum(Var)
    -> agg_ref<Name, fold_form::sum, Var, void> {
    static_assert(detail::is_numeric_var<Var>(),
        "bumbledb sum(): the input is a numeric (u64/i64) variable or "
        "r.duration(interval variable) — sum over bool is refused");
    return {};
}

/// A named min over an orderable variable or the measure.
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto min(measure_ref<Var>)
    -> agg_ref<Name, fold_form::min, measure_ref<Var>, void> {
    return {};
}

template<fixed_string Name, class Var>
[[nodiscard]] consteval auto min(Var)
    -> agg_ref<Name, fold_form::min, Var, void> {
    static_assert(detail::is_orderable_var<Var>(),
        "bumbledb min(): the input is an orderable (bool/u64/i64) "
        "variable or r.duration(interval variable)");
    return {};
}

/// A named max over an orderable variable or the measure.
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto max(measure_ref<Var>)
    -> agg_ref<Name, fold_form::max, measure_ref<Var>, void> {
    return {};
}

template<fixed_string Name, class Var>
[[nodiscard]] consteval auto max(Var)
    -> agg_ref<Name, fold_form::max, Var, void> {
    static_assert(detail::is_orderable_var<Var>(),
        "bumbledb max(): the input is an orderable (bool/u64/i64) "
        "variable or r.duration(interval variable)");
    return {};
}

/// The nullary count — `bdb::count<"n">()`: |the group's set of distinct
/// full bindings|, u64.
template<fixed_string Name>
[[nodiscard]] consteval auto count()
    -> agg_ref<Name, fold_form::count, void, void> {
    return {};
}

/// `bdb::count_distinct<"n">(vars.owner)`: |distinct values of one bound
/// variable within the group|, u64.
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto count_distinct(Var)
    -> agg_ref<Name, fold_form::count_distinct, Var, void> {
    static_assert(detail::is_qvar_v<Var>,
        "bumbledb count_distinct(): the argument must be a query "
        "variable (vars.field)");
    return {};
}

/// Arg-restriction toward the maximum of `key` (`ir::AggOp::ArgMax`):
/// carries `value` from the group's key-maximal bindings.
template<fixed_string Name, class Value, class Key>
[[nodiscard]] consteval auto arg_max(Value, Key)
    -> agg_ref<Name, fold_form::arg_max, Value, Key> {
    static_assert(detail::is_qvar_v<Value>,
        "bumbledb arg_max(): the carried value must be a query variable");
    static_assert(detail::is_orderable_var<Key>()
            || detail::is_measure_ref_v<Key>,
        "bumbledb arg_max(): the key must be an orderable variable or "
        "r.duration(interval variable)");
    return {};
}

/// Arg-restriction toward the minimum of `key`; rules as arg_max.
template<fixed_string Name, class Value, class Key>
[[nodiscard]] consteval auto arg_min(Value, Key)
    -> agg_ref<Name, fold_form::arg_min, Value, Key> {
    static_assert(detail::is_qvar_v<Value>,
        "bumbledb arg_min(): the carried value must be a query variable");
    static_assert(detail::is_orderable_var<Key>()
            || detail::is_measure_ref_v<Key>,
        "bumbledb arg_min(): the key must be an orderable variable or "
        "r.duration(interval variable)");
    return {};
}

/// The coalescing fold (`ir::AggOp::Pack`) — `bdb::pack<"free">(span)`:
/// the maximal disjoint half-open segments of the union of the group's
/// interval point sets — RELATION-SHAPED, one answer row per (group,
/// maximal segment). At most one pack per find, never beside another
/// aggregate (judged at rule assembly).
template<fixed_string Name, class Var>
[[nodiscard]] consteval auto pack(Var)
    -> agg_ref<Name, fold_form::pack, Var, void> {
    static_assert(detail::is_qvar_v<Var>
            && (Var::cls.kind == value_kind::interval_u64
                || Var::cls.kind == value_kind::interval_i64),
        "bumbledb pack(): the input must be an interval-typed query "
        "variable — pack coalesces interval point sets");
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
        static_assert(detail::is_query_member<Facade>(),
            "bumbledb r.vars(): the argument must be a relation facade "
            "(bdb::relation<...> or bdb::closed<...>)");
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
        -> agg_ref<Name, fold_form::sum, measure_ref<Var>, void> {
        return {};
    }

    /// Starts the rule body with one positive EDB atom.
    template<class Facade>
    [[nodiscard]] consteval auto match(Facade facade,
        match_pattern_of<S, Facade> const& pattern) const
        -> rule_chain<S, Facade> {
        return rule_chain<S>{}.match(facade, pattern);
    }

    /// Starts the rule body with one POSITIVE recursive atom (an idb atom
    /// grounds its variables — the finished set's identity projection
    /// needs no re-grounding join).
    template<fixed_string Name, class... Binds>
    [[nodiscard]] consteval auto idb(pred_tag<Name> tag,
        Binds... binds) const -> rule_chain<S> {
        return rule_chain<S>{}.idb(tag, binds...);
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

// ————————————————————————————————————————————————————————————————————
// Stratified recursion (ts/src/query/predicate.ts; lowering.md §4).
// ————————————————————————————————————————————————————————————————————

/// One recursive predicate's DEFINITION: the name plus its rule builders
/// (evaluated inside bdb::program, where the schema is known). Rule 0
/// seals the head signature; every later rule derives the same head.
template<fixed_string Name, class... Builds>
struct rec_def {
    std::tuple<Builds...> builds;
};

/// Declares one recursive predicate — `bdb::rec<"reach">(rule0, rule1)`.
/// Declaration order inside bdb::program = the dense PredId.
template<fixed_string Name, class... Builds>
[[nodiscard]] consteval auto rec(Builds... builds)
    -> rec_def<Name, Builds...> {
    static_assert(sizeof...(Builds) >= 1,
        "bumbledb rec(): a predicate with no defining clause seals no "
        "signature — give the rec at least one rule");
    static_assert(sizeof...(Builds) <= max_query_rules,
        "bumbledb rec(): too many rules for one predicate");
    return {std::tuple{builds...}};
}

/// The OUTPUT predicate's definition (one rule per build; multiple rules
/// = set union). Must be bdb::program's LAST argument.
template<class... Builds>
struct output_def {
    std::tuple<Builds...> builds;
};

template<class... Builds>
[[nodiscard]] consteval auto output(Builds... builds)
    -> output_def<Builds...> {
    static_assert(sizeof...(Builds) >= 1,
        "bumbledb output(): the output needs at least one rule");
    static_assert(sizeof...(Builds) <= max_query_rules,
        "bumbledb output(): too many output rules");
    return {std::tuple{builds...}};
}

} // namespace bdb

namespace bdb::detail {

template<class T>
inline constexpr bool is_rec_def_v = false;

template<fixed_string Name, class... Builds>
inline constexpr bool is_rec_def_v<rec_def<Name, Builds...>> = true;

template<class T>
inline constexpr bool is_output_def_v = false;

template<class... Builds>
inline constexpr bool is_output_def_v<output_def<Builds...>> = true;

template<class T>
struct rec_name_of_t;

template<fixed_string Name, class... Builds>
struct rec_name_of_t<rec_def<Name, Builds...>> {
    static constexpr name_text value = to_name_text(Name.view());
};

/// Evaluates one rule builder under the schema's rule scope.
template<class S, class Build>
consteval auto built_rule(Build const& build) -> rule_data {
    auto const result = build(rule_scope<S>{});
    static_assert(
        std::same_as<std::remove_cvref_t<decltype(result)>, rule_data>,
        "bumbledb program rule: the rule body must end in .find(...)");
    return result;
}

/// One predicate's evaluated rules.
struct built_pred {
    name_text name;
    std::size_t rule_count;
    std::array<rule_data, max_query_rules> rules;
};

template<class S, class Part>
consteval auto built_pred_of(Part const& part) -> built_pred {
    auto out = built_pred{};
    out.name = rec_name_of_t<Part>::value;
    auto const add = [&](rule_data const& rule) {
        out.rules[out.rule_count] = rule;
        ++out.rule_count;
    };
    std::apply(
        [&](auto const&... builds) {
            (add(built_rule<S>(builds)), ...);
        },
        part.builds);
    return out;
}

} // namespace bdb::detail

export namespace bdb {

/// The stratified-program entry point (the TS `program(S, p => ...)`):
/// `bdb::program(S, bdb::rec<"reach">(rule...), ..., bdb::output(rule...))`.
/// Recs in declaration order (PredId = index), the output predicate last
/// (`output = rec count` — lowering.md §4.2). Each rec's rule 0 seals its
/// head; a rec's rules may `.idb` ONLY the rec itself (the self-recursion
/// cut) and project bound variables only; output rules join/negate any
/// FINISHED stratum and aggregate freely. The param registry folds the
/// recs' uses first (declaration order, rules in order), then the
/// output's — registry order = positional bind order. The sealed program
/// is an ordinary query value: `db.prepare<Program>()`.
template<Theory S, class... Parts>
[[nodiscard]] consteval auto program(S const&, Parts const&... parts)
    -> query_value<S> {
    constexpr auto part_count = sizeof...(Parts);
    static_assert(part_count >= 1
            && (detail::is_output_def_v<
                Parts...[part_count - 1]>),
        "bumbledb program(): the LAST argument must be bdb::output(...) — "
        "the sealed program IS the query value");
    static_assert(
        ((detail::is_rec_def_v<Parts> || detail::is_output_def_v<Parts>)
            && ...),
        "bumbledb program(): every argument after the schema is a "
        "bdb::rec<\"name\">(rules...) or the final bdb::output(rules...)");
    constexpr auto rec_total = (std::size_t{0} + ...
        + (detail::is_rec_def_v<Parts> ? 1U : 0U));
    static_assert(rec_total + 1 == part_count,
        "bumbledb program(): bdb::output(...) is declared once, last");
    if constexpr (rec_total > max_program_recs) {
        detail::program_has_too_many_recs();
    }

    // 1. Evaluate every rule builder (recs in declaration order, output
    //    last) under the schema's rule scope.
    auto recs = std::array<detail::built_pred, rec_total == 0 ? 1 : rec_total>{};
    auto output_rules = detail::built_pred{};
    {
        auto rec_index = std::size_t{0};
        auto const eat = [&]<class Part>(Part const& part) {
            if constexpr (detail::is_rec_def_v<Part>) {
                recs[rec_index] = detail::built_pred_of<S>(part);
                ++rec_index;
            } else {
                auto out = detail::built_pred{};
                auto const add = [&](rule_data const& rule) {
                    out.rules[out.rule_count] = rule;
                    ++out.rule_count;
                };
                std::apply(
                    [&](auto const&... builds) {
                        (add(detail::built_rule<S>(builds)), ...);
                    },
                    part.builds);
                output_rules = out;
            }
        };
        (eat(parts), ...);
    }

    // 2. Distinct rec names; sealed heads (rule 0 of each rec) with the
    //    recursive-head roster wall (bound variables only — judged in
    //    lower_rule) and the alignment wall across each rec's rules.
    auto out = query_value<S>{};
    auto& ir = out.ir;
    ir.rec_count = rec_total;
    for (auto index = std::size_t{0}; index != rec_total; ++index) {
        for (auto other = std::size_t{0}; other != index; ++other) {
            if (recs[other].name == recs[index].name) {
                detail::program_rec_names_must_be_distinct();
            }
        }
        if (recs[index].rule_count == 0) {
            detail::program_rec_defines_at_least_one_rule();
        }
        auto const& seal = recs[index].rules[0];
        ir.recs[index].head_name = recs[index].name;
        ir.recs[index].head_count = seal.find_count;
        for (auto column = std::size_t{0}; column != seal.find_count;
            ++column) {
            ir.recs[index].head[column] = seal.finds[column];
        }
        for (auto rule = std::size_t{1}; rule != recs[index].rule_count;
            ++rule) {
            detail::align_head(ir.recs[index].head_count,
                ir.recs[index].head, recs[index].rules[rule]);
        }
    }

    // 3. The param registry fold: recs first (declaration order, rules in
    //    order), output rules last (lowering.md §4.2).
    for (auto index = std::size_t{0}; index != rec_total; ++index) {
        for (auto rule = std::size_t{0}; rule != recs[index].rule_count;
            ++rule) {
            detail::fold_uses(ir, recs[index].rules[rule].state);
        }
    }
    for (auto rule = std::size_t{0}; rule != output_rules.rule_count;
        ++rule) {
        detail::fold_uses(ir, output_rules.rules[rule].state);
    }

    // 4. Lower every rule (rec rules under the self-recursion cut; the
    //    output rules under the finished-strata rules), then seal the
    //    output head.
    for (auto index = std::size_t{0}; index != rec_total; ++index) {
        ir.recs[index].rule_count = recs[index].rule_count;
        for (auto rule = std::size_t{0}; rule != recs[index].rule_count;
            ++rule) {
            ir.recs[index].rules[rule] =
                detail::lower_rule(ir, recs[index].rules[rule], index);
        }
    }
    for (auto rule = std::size_t{0}; rule != output_rules.rule_count;
        ++rule) {
        auto const& data = output_rules.rules[rule];
        auto const lowered = detail::lower_rule(ir, data, detail::no_rec);
        if (rule == 0) {
            ir.head_count = data.find_count;
            for (auto column = std::size_t{0};
                column != data.find_count; ++column) {
                ir.head[column] = data.finds[column];
            }
        } else {
            detail::align_head(ir.head_count, ir.head, data);
        }
        ir.rules[ir.rule_count] = lowered;
        ++ir.rule_count;
    }
    return out;
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

/// A runtime SET param's member type: a borrowed SEQUENCE of the
/// anchored element type (the empty span is legal and matches nothing —
/// the engine's rule). Borrow contract: the span (and any string/bytes
/// elements) must stay alive for the execute call only — the bridge
/// copies before returning.
consteval auto set_type_of(field_class cls) -> std::meta::info {
    switch (cls.kind) {
    case value_kind::boolean:
        return query_type_reflection<std::span<bool const>>;
    case value_kind::u64:
        return query_type_reflection<std::span<std::uint64_t const>>;
    case value_kind::i64:
        return query_type_reflection<std::span<std::int64_t const>>;
    case value_kind::string:
        return query_type_reflection<std::span<std::string_view const>>;
    case value_kind::fixed_bytes:
        return query_type_reflection<
            std::span<std::span<std::byte const> const>>;
    case value_kind::interval_u64:
        return query_type_reflection<
            std::span<interval<std::uint64_t> const>>;
    case value_kind::interval_i64:
        break;
    }
    return query_type_reflection<std::span<interval<std::int64_t> const>>;
}

consteval auto param_type_of(param_data const& parameter)
    -> std::meta::info {
    if (parameter.shape == param_shape::mask) {
        return query_type_reflection<allen_mask>;
    }
    if (parameter.shape == param_shape::set) {
        // The runtime ∈-set lane (TODO_CPP §21): the member is a span of
        // the anchored element type. (Membership entries never reach
        // here — the product synthesis skips them.)
        return set_type_of(parameter.domain);
    }
    // The scalar lane: the anchored domain IS the member type — a
    // point-domain param is its element.
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
/// compile error (TODO_CPP §21). MEMBERSHIP entries are skipped — their
/// sets were pre-resolved at build, and execution injects the frozen set
/// positionally (the execute-time params object is never consulted for
/// them; ts/src/query/run.ts:57-63).
template<auto Query>
struct query_params_types {
    struct Params;
    consteval {
        auto specs = std::vector<std::meta::info>{};
        for (auto index = std::size_t{0}; index != Query.ir.param_count;
            ++index) {
            if (Query.ir.params[index].membership) {
                continue;
            }
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
