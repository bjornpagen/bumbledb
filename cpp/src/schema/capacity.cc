// :capacity — the capacity law and its vocabulary: weigh / within / ref /
// duration, then the law itself (TODO_CPP §9; lowering.md §1.7; the
// operator read order C2: target, weight, window, source).
export module bumbledb:capacity;

import std;
import :name;
import :classify;
import :spec;
import :schema_member;
import :face;
import :contained;

export namespace bdb {

// ————————————————————————————————————————————————————————————————————
// capacity: weigh / within / ref / duration, then the law itself.
// ————————————————————————————————————————————————————————————————————

/// `duration(coord)` — an interval column read as its measure (a
/// weigh-able quantity, and a dependent hi bound; lowering.md §1.7).
template<class Coordinate>
struct duration_measure {};

template<class Coordinate>
[[nodiscard]] consteval auto duration(Coordinate) -> duration_measure<Coordinate> {
	static_assert(detail::is_coordinate_v<Coordinate>, "bumbledb duration(): the argument must be a relation coordinate "
	                                                   "(Relation.field)");
	static_assert(Coordinate::kind == value_kind::interval_u64 || Coordinate::kind == value_kind::interval_i64,
	              "bumbledb duration(): the coordinate must be an interval column — "
	              "a duration is an interval's measure");
	return {};
}

/// `ref(coord)` — a dependent capacity bound resolved by name against the
/// TARGET row's full roster (hi slot only, C6).
template<class Coordinate>
struct ref_bound {};

template<class Coordinate>
[[nodiscard]] consteval auto ref(Coordinate) -> ref_bound<Coordinate> {
	static_assert(detail::is_coordinate_v<Coordinate>, "bumbledb ref(): the argument must be a relation coordinate "
	                                                   "(Relation.field)");
	static_assert(Coordinate::kind == value_kind::u64, "bumbledb ref(): a dependent bound reads a std::uint64_t column "
	                                                   "of the target row");
	return {};
}

/// The unit weight (each source row weighs 1) — the no-weigh overload of
/// capacity mints it; unit is a case, never an absence (C4).
struct unit_weight {};

/// `weigh(coord)` — the weight is a u64 column of the SOURCE row.
template<class Coordinate>
struct field_weight {};

/// `weigh(duration(coord))` — the weight is a SOURCE interval's measure.
template<class Coordinate>
struct duration_weight {};

template<class Coordinate>
[[nodiscard]] consteval auto weigh(Coordinate) -> field_weight<Coordinate> {
	static_assert(detail::is_coordinate_v<Coordinate>, "bumbledb weigh(): the argument must be a relation coordinate "
	                                                   "(Relation.field) or bdb::duration(coordinate)");
	static_assert(Coordinate::kind == value_kind::u64, "bumbledb weigh(): a field weight reads a std::uint64_t column of "
	                                                   "the source row (interval columns weigh through bdb::duration)");
	return {};
}

template<class Coordinate>
[[nodiscard]] consteval auto weigh(duration_measure<Coordinate>) -> duration_weight<Coordinate> {
	return {};
}

} // namespace bdb

namespace bdb::detail {

// The :interval diagnostic convention: reaching a call to one of
// these never-defined, non-constexpr functions during constant evaluation
// is the compile error, and the name is the message (the host-side ban
// table of lowering.md §1.7 — banned spellings are unwritable).
auto capacity_window_must_satisfy_lo_less_than_hi() -> void;
auto capacity_window_exact_is_spelled_within_n() -> void;
auto capacity_floor_zero_is_vacuous_delete_the_statement() -> void;
auto capacity_unit_floor_one_is_the_bare_containment() -> void;
auto capacity_unit_count_bounded_by_duration_mixes_dimensions() -> void;

} // namespace bdb::detail

export namespace bdb {

/// A capacity window value; `HiCoordinate` is the dependent hi bound's
/// coordinate (`void` for a literal hi).
template<class HiCoordinate>
struct capacity_window {
	window_data data;
};

/// `within(n)` — exactly n.
[[nodiscard]] consteval auto within(std::uint64_t exact) -> capacity_window<void> {
	return {window_data{
	    .form = window_form::exact,
	    .lo = bound_data{.form = bound_form::lit, .lit = exact, .field = name_text{}},
	    .hi = bound_data{},
	}};
}

/// `within(lo, hi)` — the half-open-count range lo..hi. The banned
/// spellings (`hi < lo`, `n..n`, `0..0`) are unwritable host-side; the
/// engine remains the wall.
[[nodiscard]] consteval auto within(std::uint64_t lo, std::uint64_t hi) -> capacity_window<void> {
	if (hi < lo) {
		detail::capacity_window_must_satisfy_lo_less_than_hi();
	}
	if (hi == lo) {
		detail::capacity_window_exact_is_spelled_within_n();
	}
	return {window_data{
	    .form = window_form::range,
	    .lo = bound_data{.form = bound_form::lit, .lit = lo, .field = name_text{}},
	    .hi = bound_data{.form = bound_form::lit, .lit = hi, .field = name_text{}},
	}};
}

/// The unbounded-above marker: `within(lo, bdb::unbounded)` is the floor
/// window (the TS `within(lo, "*")`). `{0..*}` is vacuous and refused at
/// construction; `{1..*}` on the UNIT instance is the bare containment
/// respelled and refused at capacity() (weight-sensitive — legal weighed).
struct unbounded_t {};

inline constexpr auto unbounded = unbounded_t{};

/// `within(lo, bdb::unbounded)` — at least lo, no ceiling (floor).
[[nodiscard]] consteval auto within(std::uint64_t lo, unbounded_t) -> capacity_window<void> {
	if (lo == 0) {
		detail::capacity_floor_zero_is_vacuous_delete_the_statement();
	}
	return {window_data{
	    .form = window_form::floor,
	    .lo = bound_data{.form = bound_form::lit, .lit = lo, .field = name_text{}},
	    .hi = bound_data{},
	}};
}

/// `within(lo, ref(coord))` — a dependent hi bound (target row's u64).
template<class Coordinate>
[[nodiscard]] consteval auto within(std::uint64_t lo, ref_bound<Coordinate>) -> capacity_window<Coordinate> {
	return {window_data{
	    .form = window_form::range,
	    .lo = bound_data{.form = bound_form::lit, .lit = lo, .field = name_text{}},
	    .hi = bound_data{.form = bound_form::field, .lit = 0, .field = Coordinate::field_name},
	}};
}

/// `within(lo, duration(coord))` — a dependent hi bound (target
/// interval's measure).
template<class Coordinate>
[[nodiscard]] consteval auto within(std::uint64_t lo, duration_measure<Coordinate>) -> capacity_window<Coordinate> {
	return {window_data{
	    .form = window_form::range,
	    .lo = bound_data{.form = bound_form::lit, .lit = lo, .field = name_text{}},
	    .hi = bound_data{.form = bound_form::duration_field, .lit = 0, .field = Coordinate::field_name},
	}};
}

/// A stored capacity law value: target, weight, window, source (the
/// operator read order, C2). The window's numeric payload and the faces'
/// σ selections are the value-borne data of the statement algebra.
template<class Target, class Weight, class Source>
struct capacity_law {
	using target_face = Target;
	using source_face = Source;
	using weight_type = Weight;

	Target target;
	Source source;
	window_data window;
};

} // namespace bdb

namespace bdb::detail {

template<class T>
inline constexpr bool is_capacity_v = false;

template<class Target, class Weight, class Source>
inline constexpr bool is_capacity_v<capacity_law<Target, Weight, Source>> = true;

// Weight field name + form, one reader per case.
struct weight_shape {
	weight_form form;
	name_text field;
};

[[nodiscard]] consteval auto shape_of_weight(unit_weight) -> weight_shape {
	return {weight_form::unit, name_text{}};
}

template<class Coordinate>
[[nodiscard]] consteval auto shape_of_weight(field_weight<Coordinate>) -> weight_shape {
	return {weight_form::field, Coordinate::field_name};
}

template<class Coordinate>
[[nodiscard]] consteval auto shape_of_weight(duration_weight<Coordinate>) -> weight_shape {
	return {weight_form::duration_field, Coordinate::field_name};
}

// The weight coordinate's owner (empty for unit), for the source-roster
// membership check.
[[nodiscard]] consteval auto weight_owner(unit_weight) -> name_text {
	return name_text{};
}

template<class Coordinate>
[[nodiscard]] consteval auto weight_owner(field_weight<Coordinate>) -> name_text {
	return Coordinate::relation_name;
}

template<class Coordinate>
[[nodiscard]] consteval auto weight_owner(duration_weight<Coordinate>) -> name_text {
	return Coordinate::relation_name;
}

template<class Target, class Weight, class Source>
[[nodiscard]] consteval auto capacity_weight_message() -> std::string {
	return "bumbledb capacity(): the weight must read the SOURCE row — "
	       "the source face is \"" +
	       std::string{Source::relation_name.view()} + "\" but the weigh() coordinate belongs to another relation";
}

template<class Target, class HiCoordinate>
[[nodiscard]] consteval auto capacity_bound_message() -> std::string {
	return "bumbledb capacity(): a dependent bound resolves against the "
	       "TARGET row — the target face is \"" +
	       std::string{Target::relation_name.view()} + "\" but the bound coordinate is \"" + coordinate_label<HiCoordinate>() + "\"";
}

} // namespace bdb::detail

export namespace bdb {

/// `capacity(target, weigh(...), within(...), source)` — the weighed law.
template<class Target, class Weight, class HiCoordinate, class Source>
[[nodiscard]] consteval auto capacity(Target target, Weight, capacity_window<HiCoordinate> window, Source source)
    -> capacity_law<Target, Weight, Source> {
	static_assert(detail::is_face_v<Target> && detail::is_face_v<Source>, "bumbledb capacity(): target and source must be faces — spell "
	                                                                      "them bdb::on(Relation.field, ...)");
	static_assert(Source::width == Target::width, detail::arity_message<Target, Source>("capacity"));
	static_assert(std::same_as<Weight, unit_weight> || detail::weight_owner(Weight{}) == Source::relation_name,
	              detail::capacity_weight_message<Target, Weight, Source>());
	if constexpr (!std::same_as<HiCoordinate, void>) {
		static_assert(HiCoordinate::relation_name == Target::relation_name, detail::capacity_bound_message<Target, HiCoordinate>());
	}
	return {target, source, window.data};
}

/// `capacity(target, within(...), source)` — the unit weight (C4). The
/// weight-SENSITIVE window bans run here (the TS capacity() unit-overload
/// rows): `{1..*}` on the unit instance says only what the bare
/// containment says, and a count of facts bounded by a span of time mixes
/// dimensions (C18) — both LEGAL on the weighed overload.
template<class Target, class HiCoordinate, class Source>
[[nodiscard]] consteval auto capacity(Target target, capacity_window<HiCoordinate> window, Source source) -> capacity_law<Target, unit_weight, Source> {
	if (window.data.form == window_form::floor && window.data.lo.form == bound_form::lit && window.data.lo.lit == 1) {
		detail::capacity_unit_floor_one_is_the_bare_containment();
	}
	if (window.data.hi.form == bound_form::duration_field) {
		detail::capacity_unit_count_bounded_by_duration_mixes_dimensions();
	}
	return capacity(target, unit_weight{}, window, source);
}

} // namespace bdb
