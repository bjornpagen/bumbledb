export module bumbledb:ir;

import std;
import :name;
import :classify;
import :spec;

export namespace bdb {

/** Builder capacities: SDK bounds only — the engine's own caps are far higher. */
inline constexpr std::size_t max_query_rules = 4;
inline constexpr std::size_t max_query_atoms = 8;
inline constexpr std::size_t max_query_conditions = 8;
inline constexpr std::size_t max_query_finds = 8;
inline constexpr std::size_t max_query_params = 8;
inline constexpr std::size_t max_query_vars = 32;

/**
 * Most recursive predicates one program may declare (the engine's own
 * MAX_PREDICATES is 16; lowering.md §4.1).
 */
inline constexpr std::size_t max_program_recs = 4;

/** Most handles one closed-membership array may spell in a match record. */
inline constexpr std::size_t max_membership_handles = 8;

/**
 * One structural literal payload (match/comparison literals). Strings
 * and bytes are deliberately absent: a query value must stay structural
 * (NTTP-usable) — bind such values through params instead.
 */
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

/**
 * A term's form (`ir::Term`, lowering.md §4.1). `absent` is the pattern
 * wildcard — an unmentioned field binds nothing. `param_set` is the ∈-set
 * binding: a closed-membership array lowers to it over a synthetic
 * content-addressed registry entry whose set is a program constant, never
 * carried by the execute-time params product (lowering.md §4.2).
 */
enum class query_term_form : std::uint8_t {
	absent,
	variable,
	param,
	param_set,
	literal,
	measure,
};

/**
 * One builder-stage term: variables/measures ride their mint coordinate
 * (the identity `v(Relation).field` established), params their name. A
 * membership term additionally carries its pre-resolved handle row ids —
 * queries resolve handles host-side (lowering.md §7.8).
 */
struct term_data {
	query_term_form form;
	coord_ref variable;
	name_text param;
	query_literal literal;
	std::size_t member_count;
	std::array<std::uint64_t, max_membership_handles> members;
};

/** One pattern binding as recorded: the sealed field ordinal + the term. */
struct binding_data {
	std::size_t field;
	term_data term;
};

/**
 * One EDB atom as recorded: the relation's declaration ordinal (the wire
 * RelationId — lowering.md §1.1) and the bindings in written order.
 */
struct atom_data {
	std::uint32_t relation;
	std::size_t binding_count;
	std::array<binding_data, max_relation_fields> bindings;
};

/** The comparison operators the surface mints (`ir::CmpOp`). */
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

/**
 * One leaf condition. `point_in` stores interval-LEFT, point-RIGHT
 * whatever the surface argument order; `mask` is the literal 13-bit Allen
 * word (allen conditions only).
 */
struct condition_data {
	query_cmp op;
	std::uint16_t mask;
	term_data lhs;
	term_data rhs;
};

/**
 * One named binding of a recursive (IDB) atom: the target head column by
 * name, the variable (IDB bindings are variable terms only), and the
 * variable's class facts for the head-slot join wall.
 */
struct idb_bind_data {
	name_text column;
	coord_ref variable;
	field_class cls;
	bool classed;
	coord_ref law;
};

/**
 * One recursive atom as recorded: the rec's name (resolved to its dense
 * PredId at program assembly), polarity, and the named bindings. The
 * binds are placed and numbered in the target's head order at assembly —
 * `FieldId(i)` = head position i (lowering.md §4.2).
 */
struct idb_atom_data {
	name_text pred;
	bool negated;
	std::size_t bind_count;
	std::array<idb_bind_data, max_query_finds> binds;
};

/**
 * One rule-body item: the written interleave of match/where is preserved
 * so variable numbering walks body items in written order (lowering.md
 * §4.2), whatever bucket each item later lowers into.
 */
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

/**
 * A find column's form (`ir::FindTerm`): a projected variable, a
 * var-scoped aggregate, or an aggregate over the measure.
 */
enum class find_form : std::uint8_t {
	variable,
	aggregate,
	aggregate_measure,
};

/** The aggregate ops the heads mint (`ir::AggOp`). */
enum class fold_form : std::uint8_t {
	sum,
	min,
	max,
	count,
	pack,
};

/**
 * One find column: the answer column name, the term shape, the answer
 * cell's structural class (the row-product synthesis input), and —
 * variable finds — the column's law class (the IDB head-slot join wall's
 * data).
 */
struct find_data {
	name_text name;
	find_form form;
	fold_form op;
	term_data over;
	field_class answer;
	bool has_over;
	bool classed;
	coord_ref law;
};

/** A param's wire shape (lowering.md §4.2's registry entry). */
enum class param_shape : std::uint8_t {
	value,
	set,
};

/**
 * One registered parameter: name, shape, the field-anchored bind domain
 * (the params-product member type AND the wire tag), and whether the
 * anchoring use was point-domain (an interval field's element under
 * point_in). A membership entry is a synthetic content-addressed set
 * param pre-resolved at build: it never appears in the params product,
 * and execution supplies its frozen set positionally from the query
 * constant.
 */
struct param_data {
	name_text name;
	param_shape shape;
	field_class domain;
	bool point;
	bool membership;
	std::size_t member_count;
	std::array<std::uint64_t, max_membership_handles> members;
};

/** One param use, recorded at the position that anchors it. */
struct param_use {
	name_text name;
	param_shape shape;
	field_class domain;
	bool point;
	bool membership;
	std::size_t member_count;
	std::array<std::uint64_t, max_membership_handles> members;
};

/** One rule's accumulated builder state (value tier). */
struct rule_state {
	std::size_t item_count;
	std::array<body_item, max_query_atoms + max_query_conditions> items;
	std::size_t use_count;
	std::array<param_use, max_query_params * 4> uses;
	std::size_t bound_count;
	std::array<coord_ref, max_query_vars> bound;
};

/** One completed rule: the body state plus the find head. */
struct rule_data {
	rule_state state;
	std::size_t find_count;
	std::array<find_data, max_query_finds> finds;
};

/**
 * One numbered term of the wire IR :foreign_program reads: dense
 * rule-scoped var ids, dense query-global param ids (registry order =
 * positional bind order — lowering.md §5.1).
 */
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

/**
 * One numbered atom: an EDB atom (`idb == false`, `relation` read) or a
 * recursive IDB atom (`idb == true`, `pred` read — the target's dense
 * PredId; bindings address head positions).
 */
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

/**
 * One numbered find term. `over` is read for variable/measure columns and
 * for aggregates with `has_over` (nullary `count` has none).
 */
struct wire_find {
	find_form form;
	fold_form op;
	std::uint16_t over;
	bool has_over;
};

/**
 * One numbered rule, bucketed exactly as the bridge's `bdb_rule` reads it
 * (positive atoms / negated atoms / conditions, each in written order).
 */
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

/**
 * One lowered recursive predicate: its name (idb resolution + walls), its
 * head (rule 0's finds — the sealed signature), and its numbered rules.
 */
struct pred_ir {
	name_text head_name;
	std::size_t rule_count;
	std::array<wire_rule, max_query_rules> rules;
	std::size_t head_count;
	std::array<find_data, max_query_finds> head;
};

/**
 * The whole lowered query/program: the recs in declaration order (`PredId`
 * = index), the output predicate's rules/head (a plain query is the
 * degenerate no-rec program, `output = 0` — lowering.md §4.1), plus the
 * head columns (row-product synthesis) and the param registry
 * (params-product synthesis, recs' uses folded first — §4.2).
 */
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

/**
 * One built condition value (a `.where` argument): the leaf comparison
 * plus the param uses its construction anchored.
 */
struct cond_value {
	condition_data data;
	std::size_t use_count;
	std::array<param_use, 2> uses;
};

}

namespace bdb::detail {

/**
 * Value-tier walls, this one and every declaration after it: reaching a
 * call to a never-defined non-constexpr function during constant
 * evaluation is the compile error, and the name is the message (the
 * :interval diagnostic convention).
 */
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

}
