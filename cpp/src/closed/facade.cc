/**
 * Closed relations and payload-bearing closed vocabularies (lowering.md
 * §1.3, §2, §7.3; TS reference: ts/src/closed.ts). The minted value is a
 * relation facade: `Kind.id` is a schema coordinate and a union-find
 * generator, and the ground axioms are sealed schema data (virtual
 * storage, frozen by the fingerprint); the handle constants, the axiom
 * readback, and the `bdb::ref<Kind.id>` spelling are host projections
 * for ergonomics — never replacements for the relational semantics.
 * Handle names ride the TYPE tier (bare-tier NTTPs, the payload tier's
 * `bdb::member<"...">`) because facade member names are minted by
 * define_aggregate — plain string arguments cannot mint member names.
 */
export module bumbledb:closed_facade;

import std;
import :name;
import :classify;
import :coord;
import :handle;
import :id;
import :member;
import :axioms;

export namespace bdb::detail {

/**
 * THE closed-facade discriminant: a class whose FIRST member is a
 * `closed_id` (the mint puts it there; nothing else does).
 */
[[nodiscard]] consteval auto is_closed_facade_type(std::meta::info type) -> bool {
	auto const t = std::meta::dealias(type);
	if (!std::meta::is_class_type(t)) {
		return false;
	}
	auto const members = std::meta::nonstatic_data_members_of(t, std::meta::access_context::current());
	if (members.empty()) {
		return false;
	}
	auto const first = std::meta::dealias(std::meta::type_of(members[0]));
	return std::meta::has_template_arguments(first) && std::meta::template_of(first) == ^^closed_id;
}

template<class T>
[[nodiscard]] consteval auto is_closed_facade() -> bool {
	return is_closed_facade_type(^^T);
}

}

namespace bdb::detail {

[[nodiscard]] consteval auto payload_members(std::meta::info payload) -> std::vector<std::meta::info> {
	return std::meta::nonstatic_data_members_of(payload, std::meta::access_context::current());
}

[[nodiscard]] consteval auto closed_subject(std::string_view name) -> std::string {
	return std::string{"bumbledb closed relation \""} + std::string{name} + "\"";
}

/**
 * A payload column's admissible kinds: the axiom-literal roster (bool /
 * u64 / i64 / str — the recipes' payload vocabulary).
 */
[[nodiscard]] consteval auto payload_column_supported(std::meta::info member) -> bool {
	auto const cls = classify(std::meta::type_of(member));
	if (!cls.has_value()) {
		return false;
	}
	return cls->kind == value_kind::boolean || cls->kind == value_kind::u64 || cls->kind == value_kind::i64 ||
	       cls->kind == value_kind::string;
}

template<class Payload>
[[nodiscard]] consteval auto payload_supported() -> bool {
	auto const members = payload_members(^^Payload);
	if (members.size() > max_closed_columns) {
		return false;
	}
	for (auto const member : members) {
		if (!payload_column_supported(member)) {
			return false;
		}
		if (is_fresh_marked(member)) {
			return false;
		}
		auto const name = wire_field_name(member);
		if (name == "id" || name == "axioms" || name == "data") {
			return false;
		}
	}
	return true;
}

template<class Payload>
[[nodiscard]] consteval auto payload_message(std::string_view name) -> std::string {
	auto const members = payload_members(^^Payload);
	if (members.size() > max_closed_columns) {
		return closed_subject(name) + ": the payload exceeds max_closed_columns";
	}
	for (auto const member : members) {
		auto const field = wire_field_name(member);
		if (!payload_column_supported(member)) {
			return closed_subject(name) + ": payload column \"" + field + "\" has unsupported type '" +
			       std::string{std::meta::display_string_of(std::meta::type_of(member))} +
			       "' — closed payload columns are bool, std::uint64_t, "
			       "std::int64_t, or std::string";
		}
		if (is_fresh_marked(member)) {
			return closed_subject(name) + ": payload column \"" + field +
			       "\" is marked [[=bdb::fresh]] — a vocabulary's rows are "
			       "ground axioms, never minted";
		}
		if (field == "id") {
			return closed_subject(name) + ": the payload column \"id\" collides with the sealed "
			                              "shape's synthetic id (ordinal 0) — name it something "
			                              "else";
		}
		if (field == "axioms" || field == "data") {
			return closed_subject(name) + ": payload column \"" + field + "\" collides with the facade's readback surface";
		}
	}
	return {};
}

template<class... Members>
[[nodiscard]] consteval auto handles_distinct() -> bool {
	auto const names = std::array<name_text, sizeof...(Members)>{Members::handle...};
	for (auto first = std::size_t{0}; first != names.size(); ++first) {
		for (auto second = first + 1; second != names.size(); ++second) {
			if (names[first] == names[second]) {
				return false;
			}
		}
	}
	return true;
}

template<class Payload, class... Members>
[[nodiscard]] consteval auto handles_avoid_facade_names() -> bool {
	auto const names = std::array<name_text, sizeof...(Members)>{Members::handle...};
	for (auto const& name : names) {
		auto const view = name.view();
		if (view == "id" || view == "axioms" || view == "data") {
			return false;
		}
		for (auto const column : payload_members(^^Payload)) {
			if (view == wire_field_name(column)) {
				return false;
			}
		}
	}
	return true;
}

template<class... Members>
[[nodiscard]] consteval auto duplicate_handle_message(std::string_view name) -> std::string {
	auto const names = std::array<name_text, sizeof...(Members)>{Members::handle...};
	for (auto first = std::size_t{0}; first != names.size(); ++first) {
		for (auto second = first + 1; second != names.size(); ++second) {
			if (names[first] == names[second]) {
				return closed_subject(name) + ": duplicate handle \"" + std::string{names[first].view()} + "\"";
			}
		}
	}
	return {};
}

template<class Payload, class... Members>
[[nodiscard]] consteval auto reserved_handle_message(std::string_view name) -> std::string {
	auto const names = std::array<name_text, sizeof...(Members)>{Members::handle...};
	for (auto const& handle : names) {
		auto const view = handle.view();
		if (view == "id" || view == "axioms" || view == "data") {
			return closed_subject(name) + ": handle \"" + std::string{view} +
			       "\" collides with the facade's own surface (id / axioms "
			       "/ data) — the C++ facade projects handles as members, "
			       "so those three names are reserved here";
		}
		for (auto const column : payload_members(^^Payload)) {
			if (view == wire_field_name(column)) {
				return closed_subject(name) + ": handle \"" + std::string{view} + "\" collides with a payload column of the same name";
			}
		}
	}
	return {};
}

/**
 * The facade synthesis (class-template-scope define_aggregate). Member
 * order is load-bearing: id, payload coordinates at sealed ordinals
 * (declared index + 1 — lowering.md §1.11), handle constants
 * (declaration order = row id), axiom readback, wire carrier. Bad input
 * never breaks the injections (duplicate names skip, unclassifiable
 * columns default) so mint_closed()'s static_asserts stay the ONE
 * diagnostic.
 */
template<fixed_string Name, class Payload, class... Members>
struct closed_types {
	/**
	 * The axiom-readback product (`Kind.axioms.DirectPass.rank`): one
	 * member per handle, typed by the payload row.
	 */
	struct Axioms;
	struct Facade;

	consteval {
		auto specs = std::vector<std::meta::info>{};
		auto used = std::vector<std::string>{};
		[[maybe_unused]] auto const add = [&](name_text handle) {
			auto const name = spec_name(handle.view());
			for (auto const& seen : used) {
				if (seen == name) {
					return;
				}
			}
			used.push_back(name);
			specs.push_back(std::meta::data_member_spec(^^Payload, {
			                                                           .name = name}));
		};
		(add(Members::handle), ...);
		std::meta::define_aggregate(^^Axioms, specs);
	}

	consteval {
		auto specs = std::vector<std::meta::info>{};
		auto used = std::vector<std::string>{spec_name("id"), spec_name("axioms"), spec_name("data")};
		auto const taken = [&](std::string const& name) {
			for (auto const& seen : used) {
				if (seen == name) {
					return true;
				}
			}
			return false;
		};

		specs.push_back(std::meta::data_member_spec(std::meta::substitute(^^closed_id,
		                                                                  {
		                                                                      std::meta::reflect_constant(to_name_text(Name.view())),
		                                                                      std::meta::reflect_constant(sizeof...(Members))}),
		                                            {.name = spec_name("id")}));

		auto ordinal = std::size_t{1};
		for (auto const column : payload_members(^^Payload)) {
			auto const cls = classify(std::meta::type_of(column)).value_or(field_class{value_kind::u64, 0});
			auto const name = spec_name(wire_field_name(column));
			if (!taken(name)) {
				used.push_back(name);
				specs.push_back(std::meta::data_member_spec(
				    std::meta::substitute(^^coord,
				                          {
				                              std::meta::type_of(column), std::meta::reflect_constant(to_name_text(Name.view())),
				                              std::meta::reflect_constant(to_name_text(wire_field_name(column))),
				                              std::meta::reflect_constant(ordinal), std::meta::reflect_constant(cls),
				                              std::meta::reflect_constant(false)}),
				    {.name = std::meta::identifier_of(column)}));
			}
			++ordinal;
		}

		auto index = std::uint64_t{0};
		[[maybe_unused]] auto const add_handle = [&](name_text handle) {
			auto const name = spec_name(handle.view());
			if (!taken(name)) {
				used.push_back(name);
				specs.push_back(std::meta::data_member_spec(
				    std::meta::substitute(^^handle_value,
				                          {
				                              std::meta::reflect_constant(to_name_text(Name.view())), std::meta::reflect_constant(handle),
				                              std::meta::reflect_constant(index)}),
				    {.name = name}));
			}
			++index;
		};
		(add_handle(Members::handle), ...);

		specs.push_back(std::meta::data_member_spec(^^Axioms, {
		                                                          .name = spec_name("axioms")}));
		specs.push_back(std::meta::data_member_spec(^^closed_info, {
		                                                               .name = spec_name("data")}));
		std::meta::define_aggregate(^^Facade, specs);
	}
};

template<fixed_string Name, class Payload, class... Members>
[[nodiscard]] consteval auto mint_closed(Members const&... members) -> typename closed_types<Name, Payload, Members...>::Facade {
	static_assert(handles_distinct<Members...>(), duplicate_handle_message<Members...>(Name.view()));
	static_assert(sizeof...(Members) <= max_closed_handles, "bumbledb closed(): the vocabulary exceeds max_closed_handles");
	static_assert(payload_supported<Payload>(), payload_message<Payload>(Name.view()));
	static_assert(handles_avoid_facade_names<Payload, Members...>(), reserved_handle_message<Payload, Members...>(Name.view()));

	using Types = closed_types<Name, Payload, Members...>;
	auto out = typename Types::Facade{};

	auto const handles = std::array<name_text, sizeof...(Members)>{Members::handle...};
	auto const payloads = std::array<Payload, sizeof...(Members)>{members.payload...};

	constexpr auto rows =
	    std::define_static_array(std::meta::nonstatic_data_members_of(^^typename Types::Axioms, std::meta::access_context::current()));
	template for (constexpr auto index : index_array<rows.size()>()) {
		out.axioms.[:rows[index]:] = payloads[index];
	}

	out.data = closed_info_of<Name, Payload>(handles, payloads);
	return out;
}

}

export namespace bdb {

/**
 * The bare tier (TS `closed("Kind", ["...", ...])`): handles only, as
 * NTTPs — `bdb::closed<"Kind", "Deterministic", "CustomOperator">()`.
 * The extension is sealed at declaration; storage is virtual; row id =
 * declaration order.
 */
template<fixed_string Name, fixed_string... Handles>
    requires(sizeof...(Handles) >= 1)
[[nodiscard]] consteval auto closed() -> typename detail::closed_types<Name, no_payload, member_value<Handles, no_payload>...>::Facade {
	return detail::mint_closed<Name, no_payload>(member_value<Handles, no_payload>{no_payload{}}...);
}

/**
 * The payload tier (TS `closed("Kind", {cols}, {axioms})`): declared
 * intrinsic columns AND ground axioms, one call —
 * `bdb::closed<"Kind", KindPayload>(bdb::member<"DirectPass">(
 * KindPayload{.mastered = true, .rank = 30}), ...)`.
 */
template<fixed_string Name, class Payload, class... Members>
    requires(sizeof...(Members) >= 1)
[[nodiscard]] consteval auto closed(Members const&... members) -> typename detail::closed_types<Name, Payload, Members...>::Facade {
	static_assert((detail::is_member_of_v<Members, Payload> && ...), "bumbledb closed(): every payload-tier argument must be a "
	                                                                 "bdb::member<\"Handle\">(Payload{...}) of THIS vocabulary's "
	                                                                 "payload type");
	return detail::mint_closed<Name, Payload>(members...);
}

}
