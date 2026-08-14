/**
 * The flattened SchemaSpec data shapes the schema elaborator fills and
 * the runtime lane lowers to the bridge (lowering.md §2–§3).
 */
export module bumbledb:spec;

import std;
import :name;
import :classify;
import :axioms;

export namespace bdb {

/**
 * One semantic coordinate by name: the class-map currency ("Service.id"
 * as data). Structural and NTTP-friendly like everything here.
 */
struct coord_ref {
	name_text relation;
	name_text field;

	[[nodiscard]] constexpr auto operator==(coord_ref const&) const -> bool = default;
};

/**
 * One declared field of the flattened relation table. `width` is the
 * fixed-width interval label (0 = the general interval — lowering.md
 * §1.8; a fingerprint input on the wire).
 */
struct field_data {
	name_text name;
	value_kind kind;
	std::uint16_t fixed_len;
	std::uint64_t width;
	bool fresh;
};

/**
 * One relation of the flattened table, declaration order throughout.
 * A CLOSED member's `fields` are its SEALED roster — the synthetic `id`
 * at index 0, declared payload columns shifted +1 (lowering.md §1.11);
 * the wire lane skips index 0 and reads the parallel closed table for
 * the sealed extension (declared columns only cross as FieldSpecs — §7.3).
 * Ordinary relations do not carry a closed payload (std::variant /
 * std::optional are not NTTP-structural on this toolchain).
 */
struct relation_data {
	name_text name;
	std::size_t field_count;
	std::array<field_data, max_extension_rows> fields;
};

/**
 * One σ/ψ literal as spelled: a handle crosses BY NAME on the schema
 * wire (the ENGINE resolves it — lowering.md §7.8); a value literal
 * crosses tagged.
 */
struct selection_literal {
	bool is_handle;
	name_text handle;
	value_kind kind;
	bool boolean;
	std::uint64_t u64;
	std::int64_t i64;
	name_text text;
};

/**
 * One σ binding: `field == literal-or-set` (read conjunctively across a
 * face's bindings; a binding's ≥2 literals read disjunctively).
 */
struct selection_data {
	name_text field;
	std::size_t literal_count;
	std::array<selection_literal, max_extension_rows> literals;
};

/**
 * One lowered statement face: relation + written projection + the σ/ψ
 * selection (lowered AS-IS, never pre-folded — the engine folds against
 * the sealed extension at validate; lowering.md §2).
 */
struct side_data {
	name_text relation;
	std::size_t width;
	std::array<name_text, max_extension_rows> fields;
	std::size_t selection_count;
	std::array<selection_data, max_extension_rows> selections;
};

/**
 * The statement form tags (lowering.md §1.9; `key` lowers as fd).
 */
enum class statement_form : std::uint8_t {
	key,
	containment,
	capacity,
};

/**
 * A capacity weight's form (unit is a case, never an absence — C4).
 */
enum class weight_form : std::uint8_t {
	unit,
	field,
	duration_field,
};

enum class bound_form : std::uint8_t {
	lit,
	field,
	duration_field,
};

enum class window_form : std::uint8_t {
	exact,
	range,
	floor,
};

struct bound_data {
	bound_form form;
	std::uint64_t lit;
	name_text field;
};

struct window_data {
	window_form form;
	bound_data lo;
	bound_data hi;
};

/**
 * One declared statement, flattened for the wire lane. `key` uses
 * `source` for its relation/projection; capacity reads target, weight,
 * window, source (the operator read order, C2).
 */
struct statement_data {
	statement_form form;
	side_data source;
	side_data target;
	bool bidirectional;
	weight_form weight;
	name_text weight_field;
	window_data window;
};

/**
 * One coordinate's law-computed class: absent (`classed == false`) on a
 * field in no law; otherwise the class-naming coordinate (generator
 * first, else least member in relation-declaration × field-declaration
 * order — lowering.md §3.5).
 */
struct class_entry {
	coord_ref coordinate;
	bool classed;
	coord_ref class_name;
};

}
