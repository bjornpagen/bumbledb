export module bumbledb:where;

import std;
import :name;
import :classify;
import :coord;
import :handle;
import :spec;
import :schema_member;

namespace bdb::detail {

/**
 * The :interval diagnostic convention: reaching a call to one of these
 * never-defined, non-constexpr functions during constant evaluation is
 * the compile error, and the name is the message.
 */
auto face_has_too_many_selection_bindings() -> void;
auto where_selection_binds_nothing() -> void;

/**
 * One where-pattern slot: the default state binds nothing; a literal or
 * a handle constant binds the field. The closed-reference roster wall
 * (§34: a foreign handle is rejected NAMING both vocabularies) runs on
 * the handle constructor.
 */
template<class T, name_text Relation, name_text Field, field_class Class>
struct where_slot {
	static constexpr name_text field_name = Field;

	std::optional<selection_literal> literal{};

	where_slot() = default;

	/**
	 * A scalar value literal, field-typed (`{.mastered = true}`).
	 */
	consteval where_slot(T value)
	    requires(!is_closed_ref_v<T> &&
	             (Class.kind == value_kind::boolean || Class.kind == value_kind::u64 || Class.kind == value_kind::i64))
	{
		auto lit = selection_literal{};
		lit.kind = Class.kind;
		if constexpr (Class.kind == value_kind::boolean) {
			lit.boolean = value;
		} else if constexpr (Class.kind == value_kind::u64) {
			lit.u64 = value;
		} else {
			lit.i64 = value;
		}
		literal = lit;
	}

	/**
	 * A handle at a closed-reference field (`{.kind =
	 * Kind.Deterministic}`) — crosses BY NAME; the ENGINE resolves
	 * schema-lane handle literals (lowering.md §7.8).
	 */
	template<name_text HandleRoster, name_text Handle, std::uint64_t Index>
	consteval where_slot(handle_value<HandleRoster, Handle, Index>)
	    requires is_closed_ref_v<T>
	{
		static_assert(HandleRoster == T::roster_name, handle_crosses_vocabulary_message(HandleRoster, Handle, T::roster_name));
		auto lit = selection_literal{};
		lit.is_handle = true;
		lit.handle = Handle;
		literal = lit;
	}
};

/**
 * The where-pattern product of one facade: one slot per SELECTABLE
 * column — every reflected coordinate of an ordinary facade; a closed
 * facade's declared payload columns only (its synthetic id is
 * deliberately unspellable here — an id selection is spelled as handle
 * literals on the REFERENCING side, the canonical utterance).
 */
template<class Facade>
struct where_pattern_types {
	struct Pattern;
	consteval {
		auto specs = std::vector<std::meta::info>{};
		for (auto const member : std::meta::nonstatic_data_members_of(^^Facade, std::meta::access_context::current())) {
			auto const t = std::meta::dealias(std::meta::type_of(member));
			if (!std::meta::has_template_arguments(t) || std::meta::template_of(t) != ^^coord) {
				continue;
			}
			auto const args = std::meta::template_arguments_of(t);
			specs.push_back(std::meta::data_member_spec(std::meta::substitute(^^where_slot,
			                                                                  {
			                                                                      args[0], args[1], args[2], args[4]}),
			                                            {.name = std::meta::identifier_of(member)}));
		}
		std::meta::define_aggregate(^^Pattern, specs);
	}
};

/**
 * The facade's relation name (both member kinds: the first
 * coordinate-shaped member carries it).
 */
template<class Facade>
[[nodiscard]] consteval auto member_relation_of() -> name_text {
	constexpr auto members = std::define_static_array(std::meta::nonstatic_data_members_of(^^Facade, std::meta::access_context::current()));
	using First = [:std::meta::type_of(members[0]):];
	return First::relation_name;
}

template<class Facade, class First>
[[nodiscard]] consteval auto selected_projection_message() -> std::string {
	return "bumbledb on(): the ψ-selected relation is \"" + std::string{member_relation_of<Facade>().view()} +
	       "\" but the projected coordinates belong to \"" + std::string{First::relation_name.view()} +
	       "\" — a selected face projects its own relation's columns";
}

}

export namespace bdb {

/**
 * The designated-init selection pattern of one facade.
 */
template<class Facade>
using where_pattern_of = typename detail::where_pattern_types<Facade>::Pattern;

/**
 * A ψ/σ-selected face source (`bdb::where(Task, {.kind =
 * Kind.Deterministic})`): the resolved bindings, carried by VALUE into
 * `bdb::on(selected, coords...)`.
 */
template<class Facade>
struct selected {
	std::size_t selection_count{};
	std::array<selection_data, max_face_selections> selections{};
};

/**
 * Applies a σ/ψ selection to a relation for use as a statement face:
 * `bdb::on(bdb::where(Task, {.kind = Kind.Deterministic}), Task.id)`.
 * Selections change PAIRING not at all (lowering.md §3.3) — they cross
 * as the face's σ bindings, read conjunctively, resolved eagerly here
 * and lowered AS-IS, never pre-folded into an id set (the ENGINE folds
 * against the sealed extension at validate).
 */
template<class Facade>
[[nodiscard]] consteval auto where(Facade, where_pattern_of<Facade> const& pattern) -> selected<Facade> {
	static_assert(detail::is_member<Facade>(), "bumbledb where(): the first argument must be a relation facade "
	                                           "(bdb::relation<...> or bdb::closed<...>)");
	auto out = selected<Facade>{};
	using Pattern = where_pattern_of<Facade>;
	constexpr auto members =
	    std::define_static_array(std::meta::nonstatic_data_members_of(^^Pattern, std::meta::access_context::current()));
	template for (constexpr auto index : detail::index_array<members.size()>()) {
		using Slot = [:std::meta::type_of(members[index]):];
		auto const& slot = pattern.[:members[index]:];
		if (slot.literal.has_value()) {
			if (out.selection_count == max_face_selections) {
				detail::face_has_too_many_selection_bindings();
			}
			auto binding = selection_data{};
			binding.field = Slot::field_name;
			binding.literal_count = 1;
			binding.literals[0] = *slot.literal;
			out.selections[out.selection_count] = binding;
			++out.selection_count;
		}
	}
	if (out.selection_count == 0) {
		detail::where_selection_binds_nothing();
	}
	return out;
}

}
