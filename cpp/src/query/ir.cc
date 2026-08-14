export module bumbledb:ir;

import std;
import :name;
import :classify;
import :spec;

export namespace bdb {

/** Engine `MAX_RULES` (`crates/bumbledb/src/ir.rs`) — the one rule-list cap. */
inline constexpr std::size_t max_query_rules = 16;
inline constexpr std::size_t max_query_atoms = 8;
inline constexpr std::size_t max_query_conditions = 8;
inline constexpr std::size_t max_query_finds = 8;
inline constexpr std::size_t max_query_params = 8;
inline constexpr std::size_t max_query_vars = 32;

/** Most handles one closed-membership array may spell in a match record. */
inline constexpr std::size_t max_membership_handles = 8;

/**
 * One structural literal payload (match/comparison literals). Strings
 * and bytes are unwritable here: a query value must stay structural
 * (NTTP-usable) — bind such values through params instead.
 *
 * `std::variant` is not NTTP-usable on the pinned GCC (non-public bases).
 * A C union of scalars/aggregates is structural; probe recorded in sdk-011.
 */
struct query_literal {
	value_kind kind{};
	union {
		bool boolean;
		std::uint64_t u64;
		std::int64_t i64;
		struct {
			std::uint64_t start;
			std::uint64_t end;
		} u64_interval;
		struct {
			std::int64_t start;
			std::int64_t end;
		} i64_interval;
	};
};

/**
 * A term's form (`ir::Term`, lowering.md §4.1). Unmentioned pattern slots
 * never become terms — the recorded IR is a binding list. `param_set` is
 * the ∈-set binding: a closed-membership array lowers to it over a
 * synthetic content-addressed registry entry whose set is a program
 * constant, never carried by the execute-time params product
 * (lowering.md §4.2).
 */
enum class query_term_form : std::uint8_t {
	variable,
	param,
	param_set,
	literal,
	measure,
};

/** Membership-array payload of a `param_set` term. */
struct term_set {
	name_text param;
	std::size_t member_count;
	std::array<std::uint64_t, max_membership_handles> members;
};

/**
 * One recorded term: the form selects the live union arm. Pattern
 * wildcards are not terms (sdk-009).
 */
struct term_data {
	query_term_form form{};
	union {
		coord_ref variable;
		name_text param;
		term_set set;
		query_literal literal;
	};
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
 * One named binding of an interior/rec atom: the target head column by
 * name, the variable (interior bindings are variable terms only), and the
 * variable's class facts for the head-slot join wall.
 */
struct interior_bind_data {
	name_text column;
	coord_ref variable;
	field_class cls;
	coord_ref law;

	[[nodiscard]] constexpr auto classed() const -> bool {
		return law.relation.length != 0;
	}
};

/**
 * One derived-table atom as recorded: the target's name (resolved to its
 * dense InteriorId at query assembly), polarity, and the named bindings.
 * The binds are placed and numbered in the target's head order at
 * assembly — `FieldId(i)` = head position i.
 */
struct interior_atom_data {
	name_text name;
	std::size_t bind_count;
	std::array<interior_bind_data, max_query_finds> binds;
};

/**
 * One rule-body item. `std::variant` ICEs GCC 17 in consteval template
 * substitution when stored in `rule_data` (same NTTP-adjacent hole as
 * sdk-024). A C union of the live arm is structural; polarity is the
 * `body_form` tag (sdk-010).
 */
enum class body_form : std::uint8_t {
	atom,
	negated_atom,
	interior_atom,
	negated_interior,
	condition,
};

struct body_item {
	body_form form{};
	union {
		atom_data atom;
		interior_atom_data interior;
		condition_data condition;
	};
};

/**
 * A find column's form (`ir::FindTerm`): a projected variable, a
 * var-scoped aggregate, or an aggregate over the measure.
 */
enum class find_form : std::uint8_t {
	variable,
	aggregate,
	measure,
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
 * variable finds — the column's law class (the interior-head-slot join
 * wall's data).
 */
struct find_data {
	name_text name;
	find_form form;
	fold_form op;
	term_data over;
	field_class answer;
	coord_ref law;

	[[nodiscard]] constexpr auto classed() const -> bool {
		return law.relation.length != 0;
	}
};

/** A param's recorded form (lowering.md §4.2's registry entry). */
enum class param_form : std::uint8_t {
	value,
	point,
	set,
	membership,
};

/**
 * One registered parameter: name, form, and the field-anchored bind
 * domain (the params-product member type AND the wire tag). `point` is
 * an interval field's element under point_in. `membership` is a
 * synthetic content-addressed set param pre-resolved at build: it never
 * appears in the params product, and execution supplies its frozen set
 * positionally from the query constant.
 */
struct param_data {
	name_text name;
	param_form form;
	field_class domain;
	std::size_t member_count;
	std::array<std::uint64_t, max_membership_handles> members;
};

/** One param use, recorded at the position that anchors it. */
struct param_use {
	name_text name;
	param_form form;
	field_class domain;
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
 * One numbered term of the wire IR :query_view reads: dense
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
 * One numbered atom: one source tag and one payload id — EDB reads a
 * RelationId, interior reads a dense InteriorId. Bindings address head
 * positions on an interior atom.
 */
enum class atom_source : std::uint8_t {
	edb,
	interior,
};

struct wire_atom {
	atom_source source;
	std::uint32_t id;
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
 * One numbered find term. `over` is the projected var for variable/measure
 * columns and the fold input for aggregates that take one; nullary `count`
 * does not read it.
 */
struct wire_find {
	find_form form;
	fold_form op;
	std::uint16_t over;
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
 * One lowered named interior: its name (atom resolution + walls), its
 * head (rule 0's finds — the sealed signature), and its numbered rules.
 */
struct interior_ir {
	name_text name;
	std::size_t rule_count;
	std::array<wire_rule, max_query_rules> rules;
	std::size_t head_count;
	std::array<find_data, max_query_finds> head;
};

/**
 * One lowered linear rec: name, sealed head, and one pooled rule array.
 * Rec arms start at `base_count`; `base_count + rec_count` is the pool
 * against `max_query_rules` (engine `MAX_RULES`).
 */
struct rec_ir {
	name_text name;
	std::size_t head_count;
	std::array<find_data, max_query_finds> head;
	std::size_t base_count;
	std::size_t rec_count;
	std::array<wire_rule, max_query_rules> rules;
};

/**
 * One lowered query: interiors, optional rec (a member only on the
 * rec-present specialization), main rules, head, and the param registry
 * (params-product synthesis — interiors' uses folded first, then rec
 * base, then rec arms, then main). Counts `NI` / `NR` are the pack
 * lengths; `HasRec` is the rec arm.
 */
template<std::size_t NI, std::size_t NR>
struct query_body {
	std::array<interior_ir, NI> interiors{};
	std::array<wire_rule, NR> rules{};
	std::size_t head_count{};
	std::array<find_data, max_query_finds> head{};
	std::size_t param_count{};
	std::array<param_data, max_query_params> params{};
};

template<std::size_t NI, bool HasRec, std::size_t NR>
struct query_ir : query_body<NI, NR> {};

template<std::size_t NI, std::size_t NR>
struct query_ir<NI, true, NR> : query_body<NI, NR> {
	rec_ir rec{};
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
auto interior_atom_names_no_declared_table() -> void;
auto a_recursive_rule_matches_only_its_own_rec() -> void;
auto a_recursive_rule_negates_no_stratum() -> void;
auto a_recursive_rule_head_projects_bound_variables_only() -> void;
auto interior_atom_omits_a_head_column() -> void;
auto interior_atom_binds_a_name_the_head_does_not_carry() -> void;
auto interior_binding_joins_only_its_head_columns_class() -> void;
auto interior_names_must_be_distinct() -> void;
auto recursive_needs_at_least_one_base_rule() -> void;
auto recursive_needs_at_least_one_rec_rule() -> void;

}
