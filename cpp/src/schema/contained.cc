export module bumbledb:contained;

import std;
import :schema_member;
import :face;

namespace bdb::detail {

template<class Source, class Target>
[[nodiscard]] consteval auto arity_message(std::string_view constructor) -> std::string {
	return "bumbledb " + std::string{constructor} + "(): face \"" + std::string{Source::relation_name.view()} + "\" projects " +
	       render_count(Source::width) + " columns but face \"" + std::string{Target::relation_name.view()} + "\" projects " +
	       render_count(Target::width) + " — positionwise pairing requires equal arity";
}

}

export namespace bdb {

/**
 * A stored containment law value (source ⊆ target, positionwise over
 * equal-arity faces); `mirrors` is the bidirectional case and crosses as
 * ONE statement (the ENGINE performs the == split, source <= target
 * first — lowering.md §2/§7). The faces ride as VALUES: their σ/ψ
 * selections are value-borne and schema() copies them into the flattened
 * statement table.
 */
template<class Source, class Target, bool Bidirectional>
struct containment_law {
	using source_face = Source;
	using target_face = Target;
	static constexpr bool bidirectional = Bidirectional;

	Source source;
	Target target;
};

/**
 * `contained(on(Outage.service), on(Service.id))` — source ⊆ target.
 */
template<class Source, class Target>
[[nodiscard]] consteval auto contained(Source source, Target target) -> containment_law<Source, Target, false> {
	static_assert(detail::is_face_v<Source> && detail::is_face_v<Target>, "bumbledb contained(): both arguments must be faces — spell them "
	                                                                      "bdb::on(Relation.field, ...)");
	static_assert(Source::width == Target::width, detail::arity_message<Source, Target>("contained"));
	return {source, target};
}

}

namespace bdb::detail {

template<class T>
inline constexpr bool is_containment_v = false;

template<class Source, class Target, bool B>
inline constexpr bool is_containment_v<containment_law<Source, Target, B>> = true;

}
