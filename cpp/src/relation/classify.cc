/**
 * Reflection metadata is the sole field source of truth: every field
 * walk and classification derives from the row declaration; there is no
 * parallel field list anywhere.
 */
export module bumbledb:classify;

import std;
import :name;
import :interval;
import :fresh;
import :handle;

export namespace bdb {

/**
 * The structural classification of one row field — the C++ image of the
 * engine's closed ValueType roster.
 */
enum class value_kind : std::uint8_t {
	boolean,
	u64,
	i64,
	string,
	fixed_bytes,
	interval_u64,
	interval_i64,
};

/**
 * A field's classification: the kind plus the FixedBytes length (part of
 * the type and a fingerprint input; 0 elsewhere) plus the fixed-width
 * interval label (0 is the general interval; a nonzero width is a
 * fingerprint input — 75-cpp-lowering.md §1.8).
 */
struct field_class {
	value_kind kind;
	std::uint16_t fixed_len;
	std::uint64_t width = 0;

	[[nodiscard]] constexpr auto operator==(field_class const&) const -> bool = default;
};

}

namespace bdb::detail {

/* PIN(reflect-using-decl): ^^ through a template parameter — ^^std::uint64_t is ill-formed on the pinned GCC */
template<class T>
inline constexpr auto type_reflection = ^^T;

}

export namespace bdb {

/**
 * Classifies one reflected field type against the closed vocabulary;
 * nullopt = unsupported, and the caller renders the product diagnostic.
 */
[[nodiscard]] consteval auto classify(std::meta::info type) -> std::optional<field_class> {
	auto const t = std::meta::dealias(type);
	if (t == ^^bool) {
		return field_class{value_kind::boolean, 0};
	}
	if (t == detail::type_reflection<std::uint64_t>) {
		return field_class{value_kind::u64, 0};
	}
	if (t == detail::type_reflection<std::int64_t>) {
		return field_class{value_kind::i64, 0};
	}
	if (t == std::meta::dealias(detail::type_reflection<std::string>)) {
		return field_class{value_kind::string, 0};
	}
	if (!std::meta::has_template_arguments(t)) {
		return std::nullopt;
	}
	auto const tmpl = std::meta::template_of(t);
	auto const args = std::meta::template_arguments_of(t);
	if (tmpl == ^^closed_ref) {
		return field_class{value_kind::u64, 0};
	}
	if (tmpl == ^^std::array&& std::meta::dealias(args[0]) == ^^std::byte) {
		auto const len = std::meta::extract<std::size_t>(args[1]);
		if (len >= 1 && len <= 64) {
			return field_class{value_kind::fixed_bytes, static_cast<std::uint16_t>(len)};
		}
		return std::nullopt;
	}
	if (tmpl == ^^interval) {
		auto const width = std::meta::extract<std::uint64_t>(args[1]);
		if (std::meta::dealias(args[0]) == detail::type_reflection<std::uint64_t>) {
			return field_class{value_kind::interval_u64, 0, width};
		}
		return field_class{value_kind::interval_i64, 0, width};
	}
	return std::nullopt;
}

}

export namespace bdb::detail {

/**
 * Whether the member carries the `[[=bdb::fresh]]` annotation — matched
 * by the annotation's type; annotation objects reflect const.
 */
[[nodiscard]] consteval auto is_fresh_marked(std::meta::info member) -> bool {
	for (auto const annotation : std::meta::annotations_of(member)) {
		auto const type = std::meta::remove_const(std::meta::type_of(annotation));
		if (type == ^^FreshTag) {
			return true;
		}
	}
	return false;
}

/**
 * The member's WIRE field name: the `[[=bdb::named<...>]]` override when
 * present, else the reflected identifier — some cross-host wire names
 * are C++ keywords, so the identifier cannot always be the wire name.
 */
[[nodiscard]] consteval auto wire_field_name(std::meta::info member) -> std::string {
	for (auto const annotation : std::meta::annotations_of(member)) {
		auto const type = std::meta::remove_const(std::meta::type_of(annotation));
		if (type == ^^NameTag) {
			auto const tag = std::meta::extract<NameTag>(annotation);
			return spec_name(tag.name.view());
		}
	}
	return spec_name(std::meta::identifier_of(member));
}

/**
 * The row's fields, in declaration order — the one enumeration
 * everything else derives from.
 */
[[nodiscard]] consteval auto row_members(std::meta::info row) -> std::vector<std::meta::info> {
	return std::meta::nonstatic_data_members_of(row, std::meta::access_context::current());
}

[[nodiscard]] consteval auto field_count(std::meta::info row) -> std::size_t {
	return row_members(row).size();
}

[[nodiscard]] consteval auto row_is_supported(std::meta::info row) -> bool {
	for (auto const member : row_members(row)) {
		if (!classify(std::meta::type_of(member)).has_value()) {
			return false;
		}
	}
	return true;
}

[[nodiscard]] consteval auto fresh_marks_are_u64(std::meta::info row) -> bool {
	for (auto const member : row_members(row)) {
		if (!is_fresh_marked(member)) {
			continue;
		}
		auto const cls = classify(std::meta::type_of(member));
		if (!cls.has_value() || cls->kind != value_kind::u64) {
			return false;
		}
	}
	return true;
}

/**
 * Diagnostic subjects: `bumbledb relation "Service"` for the relation
 * lane, `bumbledb row type 'ServiceRow'` for the marshalling lane.
 */
[[nodiscard]] consteval auto relation_subject(std::string_view name) -> std::string {
	return std::string{"bumbledb relation \""} + spec_name(name) + "\"";
}

[[nodiscard]] consteval auto row_subject(std::meta::info row) -> std::string {
	return std::string{"bumbledb row type '"} + spec_name(std::meta::display_string_of(row)) + "'";
}

/**
 * The unsupported-field diagnostic — the compile-fail suite pins its
 * shape: the subject, the first offending field, and its type.
 */
[[nodiscard]] consteval auto unsupported_field_message(std::string subject, std::meta::info row) -> std::string {
	for (auto const member : row_members(row)) {
		if (classify(std::meta::type_of(member)).has_value()) {
			continue;
		}
		return subject + ": field \"" + spec_name(std::meta::identifier_of(member)) + "\" has unsupported row type '" +
		       spec_name(std::meta::display_string_of(std::meta::type_of(member))) +
		       "' — the value vocabulary is closed: bool, std::uint64_t, "
		       "std::int64_t, std::string, bdb::bytes<1..=64>, "
		       "bdb::interval<std::uint64_t>, bdb::interval<std::int64_t>";
	}
	return {};
}

/**
 * The misplaced-fresh diagnostic — the compile-fail suite pins its
 * shape; engine validation re-judges the u64-only rule.
 */
[[nodiscard]] consteval auto misplaced_fresh_message(std::string subject, std::meta::info row) -> std::string {
	for (auto const member : row_members(row)) {
		if (!is_fresh_marked(member)) {
			continue;
		}
		auto const cls = classify(std::meta::type_of(member));
		if (cls.has_value() && cls->kind == value_kind::u64) {
			continue;
		}
		return subject + ": field \"" + spec_name(std::meta::identifier_of(member)) + "\" is marked [[=bdb::fresh]] but has type '" +
		       spec_name(std::meta::display_string_of(std::meta::type_of(member))) + "' — fresh is legal on std::uint64_t fields only";
	}
	return {};
}

template<std::size_t Count>
[[nodiscard]] consteval auto index_array() -> std::array<std::size_t, Count> {
	auto indices = std::array<std::size_t, Count>{};
	for (auto index = std::size_t{0}; index != Count; ++index) {
		indices[index] = index;
	}
	return indices;
}

}
