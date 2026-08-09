// :coord — the compile-time semantic coordinate (TODO_CPP §6–§7).
export module bumbledb:coord;

import std;
import :name;
import :classify;

export namespace bdb {

/// One compile-time semantic coordinate (`Service.id`): relation name,
/// field name, ordinal, structural kind, and fresh mark, synthesized from
/// the reflected row declaration. Not a runtime field value.
///
/// The identity lives in the TYPE (every datum is an NTTP): two facade
/// members are two distinct coordinate types, which is what lets the
/// statement algebra (the schema partitions) carry projections, run the
/// class laws, and render §34 diagnostics naming semantic coordinates
/// entirely at compile time. Values of this type are empty structural
/// literals — coordinates stay NTTP-friendly by design.
template<class T, name_text RelationName, name_text FieldName, std::size_t Ordinal, field_class Class, bool Fresh>
struct coord {
	using value_type = T;

	static constexpr name_text relation_name = RelationName;
	static constexpr name_text field_name = FieldName;
	static constexpr std::size_t ordinal = Ordinal;
	static constexpr field_class cls = Class;
	static constexpr value_kind kind = Class.kind;
	static constexpr std::uint16_t fixed_len = Class.fixed_len;
	static constexpr bool fresh = Fresh;

	/// Name hooks (the to-string surface over the inline storage).
	[[nodiscard]] constexpr auto relation() const -> std::string_view {
		return relation_name.view();
	}

	[[nodiscard]] constexpr auto field() const -> std::string_view {
		return field_name.view();
	}
};

} // namespace bdb
