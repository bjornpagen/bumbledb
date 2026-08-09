export module bumbledb:mirrors;

import std;
import :schema_member;
import :face;
import :contained;

export namespace bdb {

/**
 * `mirrors(a, b)` — the bijection (== both ways), one statement (the
 * ENGINE performs the == split, source <= target first — lowering.md
 * §2/§7).
 */
template<class Source, class Target>
[[nodiscard]] consteval auto mirrors(Source source, Target target) -> containment_law<Source, Target, true> {
	static_assert(detail::is_face_v<Source> && detail::is_face_v<Target>, "bumbledb mirrors(): both arguments must be faces — spell them "
	                                                                      "bdb::on(Relation.field, ...)");
	static_assert(Source::width == Target::width, detail::arity_message<Source, Target>("mirrors"));
	return {source, target};
}

}
