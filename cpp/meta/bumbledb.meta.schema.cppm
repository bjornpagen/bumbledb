// bumbledb.meta.schema — the statement algebra and the consteval schema
// elaborator (TODO_CPP §9–§10, §26; lowering.md §2–§3).
//
// GCC-only reflection zone. Statements are VALUES (laws are first-class,
// §26): `bdb::key(...)`, `bdb::contained(...)`, `bdb::mirrors(...)`,
// `bdb::capacity(...)` construct literal structural values whose IDENTITY
// rides their types (the coordinates carry relation/field/kind/fresh as
// NTTPs), so `bdb::schema<"Uptime">(...)` runs the whole class-law
// computation — the union-find of lowering.md §3 — during constant
// evaluation, with §34 diagnostics that name semantic coordinates.
//
// The schema value is an NTTP-friendly literal: it exposes its relations
// as members (`Uptime.relations.Service.id`), the flattened SchemaSpec
// data the runtime lane lowers to the bridge (relations/fields in
// declaration order, DECLARED statements only in written order —
// lowering.md §2/§7), and the law-computed class map (a consteval lookup;
// class names feed newtype slots ONLY and never move the fingerprint —
// lowering.md §1.10, §3.7).
export module bumbledb.meta.schema;

import std;
import bumbledb.types;
import bumbledb.meta.relation;

export namespace bdb {

/// Widest projection a statement face may spell (a Phase-C capacity; the
/// engine's own bound is far higher).
inline constexpr std::size_t max_projection_width = 8;

/// Most declared fields one relation may carry through this elaborator.
inline constexpr std::size_t max_relation_fields = 16;

/// One semantic coordinate by name: the class-map currency ("Service.id"
/// as data). Structural and NTTP-friendly like everything here.
struct coord_ref {
    name_text relation;
    name_text field;

    constexpr auto operator==(coord_ref const&) const -> bool = default;
};

/// One declared field of the flattened relation table.
struct field_data {
    name_text name;
    value_kind kind;
    std::uint16_t fixed_len;
    bool fresh;
};

/// One relation of the flattened table, declaration order throughout.
struct relation_data {
    name_text name;
    std::size_t field_count;
    std::array<field_data, max_relation_fields> fields;
};

/// One lowered statement face: relation + written projection.
struct side_data {
    name_text relation;
    std::size_t width;
    std::array<name_text, max_projection_width> fields;
};

/// The statement form tags (lowering.md §1.9; `key` lowers as fd).
enum class statement_form : std::uint8_t {
    key,
    containment,
    capacity,
};

/// A capacity weight's form (unit is a case, never an absence — C4).
enum class weight_form : std::uint8_t {
    unit,
    field,
    duration_field,
};

/// A capacity bound's form.
enum class bound_form : std::uint8_t {
    lit,
    field,
    duration_field,
};

/// A capacity window's form.
enum class window_form : std::uint8_t {
    exact,
    range,
    floor,
};

/// One capacity bound, flattened.
struct bound_data {
    bound_form form;
    std::uint64_t lit;
    name_text field;
};

/// One capacity window, flattened.
struct window_data {
    window_form form;
    bound_data lo;
    bound_data hi;
};

/// One declared statement, flattened for the wire lane. `key` uses
/// `source` for its relation/projection; capacity reads target, weight,
/// window, source (the operator read order, C2).
struct statement_data {
    statement_form form;
    side_data source;
    side_data target;
    bool bidirectional;
    weight_form weight;
    name_text weight_field;
    window_data window;
};

/// One coordinate's law-computed class: absent (`classed == false`) on a
/// field in no law; otherwise the class-naming coordinate (generator
/// first, else least member in relation-declaration × field-declaration
/// order — lowering.md §3.5).
struct class_entry {
    coord_ref coordinate;
    bool classed;
    coord_ref class_name;
};

} // namespace bdb

namespace bdb::detail {

// ————————————————————————————————————————————————————————————————————
// Type recognition (the closed statement algebra; representation-level
// dispatch over this module's own templates).
// ————————————————————————————————————————————————————————————————————

template<class T>
inline constexpr bool is_coordinate_v = false;

template<class T, name_text R, name_text F, std::size_t O, field_class C,
    bool Fr>
inline constexpr bool is_coordinate_v<coord<T, R, F, O, C, Fr>> = true;

consteval auto is_coord_type(std::meta::info type) -> bool {
    auto const t = std::meta::dealias(type);
    return std::meta::has_template_arguments(t)
        && std::meta::template_of(t) == ^^coord;
}

/// A relation facade: a class whose every member is a coordinate (the
/// injected Coords product of bumbledb.meta.relation).
consteval auto is_facade_type(std::meta::info type) -> bool {
    auto const t = std::meta::dealias(type);
    if (!std::meta::is_class_type(t)) {
        return false;
    }
    auto const members = std::meta::nonstatic_data_members_of(
        t, std::meta::access_context::current());
    if (members.empty()) {
        return false;
    }
    for (auto const member : members) {
        if (!is_coord_type(std::meta::type_of(member))) {
            return false;
        }
    }
    return true;
}

template<class T>
consteval auto is_facade() -> bool {
    return is_facade_type(^^T);
}

/// Decimal rendering for diagnostics (std::to_string is not constexpr on
/// the pinned libstdc++).
consteval auto render_count(std::size_t value) -> std::string {
    if (value == 0) {
        return "0";
    }
    auto out = std::string{};
    while (value != 0) {
        out.insert(out.begin(),
            static_cast<char>('0' + static_cast<char>(value % 10)));
        value /= 10;
    }
    return out;
}

// The coordinate label helpers behind every §34 diagnostic.
consteval auto label(name_text relation, name_text field) -> std::string {
    return std::string{relation.view()} + "." + std::string{field.view()};
}

consteval auto quoted(name_text relation, name_text field) -> std::string {
    return "\"" + label(relation, field) + "\"";
}

template<class Coordinate>
consteval auto coordinate_label() -> std::string {
    return label(Coordinate::relation_name, Coordinate::field_name);
}

/// The first coordinate of a pack whose relation differs from First's —
/// the offender a span diagnostic names ("" when the pack is coherent).
template<class First, class... Rest>
consteval auto foreign_relation_label() -> std::string {
    auto out = std::string{};
    [[maybe_unused]] auto const check = [&]<class C>() {
        if (out.empty() && !(C::relation_name == First::relation_name)) {
            out = coordinate_label<C>();
        }
    };
    (check.template operator()<Rest>(), ...);
    return out;
}

template<class First, class... Rest>
consteval auto same_relation() -> bool {
    return ((Rest::relation_name == First::relation_name) && ...);
}

template<class First, class... Rest>
consteval auto span_message(std::string_view constructor,
    std::string_view law) -> std::string {
    return "bumbledb " + std::string{constructor}
        + "(): coordinates span two relations — \""
        + coordinate_label<First>() + "\" and \""
        + foreign_relation_label<First, Rest...>() + "\" — "
        + std::string{law};
}

} // namespace bdb::detail

export namespace bdb {

// ————————————————————————————————————————————————————————————————————
// Faces: bdb::on(coord...) — one relation, a written projection.
// ————————————————————————————————————————————————————————————————————

/// A statement face value: `on(Outage.service)`,
/// `on(Device.model, Device.watts)`. Positional pairing reads the
/// projection in written order (lowering.md §2).
template<class First, class... Rest>
struct face {
    static constexpr std::size_t width = 1 + sizeof...(Rest);
    static constexpr name_text relation_name = First::relation_name;
    static constexpr std::array<name_text, width> projection{
        First::field_name, Rest::field_name...};
};

/// Projects one or more columns of ONE relation as a statement face.
template<class First, class... Rest>
consteval auto on(First, Rest...) -> face<First, Rest...> {
    static_assert(
        detail::is_coordinate_v<First>
            && (detail::is_coordinate_v<Rest> && ...),
        "bumbledb on(): every argument must be a relation coordinate "
        "(Relation.field)");
    static_assert(detail::same_relation<First, Rest...>(),
        detail::span_message<First, Rest...>(
            "on", "a face projects one relation's columns"));
    static_assert(1 + sizeof...(Rest) <= max_projection_width,
        "bumbledb on(): the projection exceeds max_projection_width");
    return {};
}

} // namespace bdb

namespace bdb::detail {

// The key pattern injection (TODO_CPP §26): one member per projected
// coordinate, named by the field, typed by the field's physical type —
// so `db.get(Outage, outage_key, {.service = s, .window = w})` marshals
// its key values in the key statement's projection order by reflection.
template<class... Coords>
struct key_pattern_types {
    struct Pattern;
    consteval {
        std::meta::define_aggregate(^^Pattern,
            {std::meta::data_member_spec(^^typename Coords::value_type,
                {.name = spec_name(Coords::field_name.view())})...});
    }
};

} // namespace bdb::detail

export namespace bdb {

// ————————————————————————————————————————————————————————————————————
// key: the functional-dependency law (pairs NOTHING in the class laws).
// ————————————————————————————————————————————————————————————————————

/// A stored key law value (§26: the law itself is the selector — get()
/// resolves the key statement by this value's structural identity).
template<class First, class... Rest>
struct key_law {
    static constexpr std::size_t width = 1 + sizeof...(Rest);
    static constexpr name_text relation_name = First::relation_name;
    static constexpr std::array<name_text, width> projection{
        First::field_name, Rest::field_name...};

    /// The keyed-read pattern product: members named by the projected
    /// fields in projection order.
    using pattern =
        typename detail::key_pattern_types<First, Rest...>::Pattern;
};

/// `key(Outage.service, Outage.window)` — R(X) -> R over one relation.
template<class First, class... Rest>
consteval auto key(First, Rest...) -> key_law<First, Rest...> {
    static_assert(
        detail::is_coordinate_v<First>
            && (detail::is_coordinate_v<Rest> && ...),
        "bumbledb key(): every argument must be a relation coordinate "
        "(Relation.field)");
    static_assert(detail::same_relation<First, Rest...>(),
        detail::span_message<First, Rest...>(
            "key", "a key constrains one relation's own rows"));
    static_assert(1 + sizeof...(Rest) <= max_projection_width,
        "bumbledb key(): the projection exceeds max_projection_width");
    return {};
}

} // namespace bdb

namespace bdb::detail {

template<class T>
inline constexpr bool is_face_v = false;

template<class First, class... Rest>
inline constexpr bool is_face_v<face<First, Rest...>> = true;

template<class T>
inline constexpr bool is_key_v = false;

template<class First, class... Rest>
inline constexpr bool is_key_v<key_law<First, Rest...>> = true;

template<class Source, class Target>
consteval auto arity_message(std::string_view constructor) -> std::string {
    return "bumbledb " + std::string{constructor} + "(): face \""
        + std::string{Source::relation_name.view()} + "\" projects "
        + render_count(Source::width) + " columns but face \""
        + std::string{Target::relation_name.view()} + "\" projects "
        + render_count(Target::width)
        + " — positionwise pairing requires equal arity";
}

} // namespace bdb::detail

export namespace bdb {

// ————————————————————————————————————————————————————————————————————
// contained / mirrors: the containment laws.
// ————————————————————————————————————————————————————————————————————

/// A stored containment law value; `mirrors` is the bidirectional case
/// and crosses as ONE statement (the ENGINE performs the == split,
/// source <= target first — lowering.md §2/§7).
template<class Source, class Target, bool Bidirectional>
struct containment_law {
    using source_face = Source;
    using target_face = Target;
    static constexpr bool bidirectional = Bidirectional;
};

/// `contained(on(Outage.service), on(Service.id))` — source ⊆ target.
template<class Source, class Target>
consteval auto contained(Source, Target)
    -> containment_law<Source, Target, false> {
    static_assert(detail::is_face_v<Source> && detail::is_face_v<Target>,
        "bumbledb contained(): both arguments must be faces — spell them "
        "bdb::on(Relation.field, ...)");
    static_assert(Source::width == Target::width,
        detail::arity_message<Source, Target>("contained"));
    return {};
}

/// `mirrors(a, b)` — the bijection (== both ways), one statement.
template<class Source, class Target>
consteval auto mirrors(Source, Target)
    -> containment_law<Source, Target, true> {
    static_assert(detail::is_face_v<Source> && detail::is_face_v<Target>,
        "bumbledb mirrors(): both arguments must be faces — spell them "
        "bdb::on(Relation.field, ...)");
    static_assert(Source::width == Target::width,
        detail::arity_message<Source, Target>("mirrors"));
    return {};
}

// ————————————————————————————————————————————————————————————————————
// capacity: weigh / within / ref / duration, then the law itself.
// ————————————————————————————————————————————————————————————————————

/// `duration(coord)` — an interval column read as its measure (a
/// weigh-able quantity, and a dependent hi bound; lowering.md §1.7).
template<class Coordinate>
struct duration_measure {};

template<class Coordinate>
consteval auto duration(Coordinate) -> duration_measure<Coordinate> {
    static_assert(detail::is_coordinate_v<Coordinate>,
        "bumbledb duration(): the argument must be a relation coordinate "
        "(Relation.field)");
    static_assert(Coordinate::kind == value_kind::interval_u64
            || Coordinate::kind == value_kind::interval_i64,
        "bumbledb duration(): the coordinate must be an interval column — "
        "a duration is an interval's measure");
    return {};
}

/// `ref(coord)` — a dependent capacity bound resolved by name against the
/// TARGET row's full roster (hi slot only, C6).
template<class Coordinate>
struct ref_bound {};

template<class Coordinate>
consteval auto ref(Coordinate) -> ref_bound<Coordinate> {
    static_assert(detail::is_coordinate_v<Coordinate>,
        "bumbledb ref(): the argument must be a relation coordinate "
        "(Relation.field)");
    static_assert(Coordinate::kind == value_kind::u64,
        "bumbledb ref(): a dependent bound reads a std::uint64_t column "
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
consteval auto weigh(Coordinate) -> field_weight<Coordinate> {
    static_assert(detail::is_coordinate_v<Coordinate>,
        "bumbledb weigh(): the argument must be a relation coordinate "
        "(Relation.field) or bdb::duration(coordinate)");
    static_assert(Coordinate::kind == value_kind::u64,
        "bumbledb weigh(): a field weight reads a std::uint64_t column of "
        "the source row (interval columns weigh through bdb::duration)");
    return {};
}

template<class Coordinate>
consteval auto weigh(duration_measure<Coordinate>)
    -> duration_weight<Coordinate> {
    return {};
}

} // namespace bdb

namespace bdb::detail {

// The bumbledb.types diagnostic convention: reaching a call to one of
// these never-defined, non-constexpr functions during constant evaluation
// is the compile error, and the name is the message (the host-side ban
// table of lowering.md §1.7 — banned spellings are unwritable).
auto capacity_window_must_satisfy_lo_less_than_hi() -> void;
auto capacity_window_exact_is_spelled_within_n() -> void;

} // namespace bdb::detail

export namespace bdb {

/// A capacity window value; `HiCoordinate` is the dependent hi bound's
/// coordinate (`void` for a literal hi).
template<class HiCoordinate>
struct capacity_window {
    window_data data;
};

/// `within(n)` — exactly n.
consteval auto within(std::uint64_t exact) -> capacity_window<void> {
    return {window_data{
        .form = window_form::exact,
        .lo = bound_data{
            .form = bound_form::lit, .lit = exact, .field = name_text{}},
        .hi = bound_data{},
    }};
}

/// `within(lo, hi)` — the half-open-count range lo..hi. The banned
/// spellings (`hi < lo`, `n..n`, `0..0`) are unwritable host-side; the
/// engine remains the wall.
consteval auto within(std::uint64_t lo, std::uint64_t hi)
    -> capacity_window<void> {
    if (hi < lo) {
        detail::capacity_window_must_satisfy_lo_less_than_hi();
    }
    if (hi == lo) {
        detail::capacity_window_exact_is_spelled_within_n();
    }
    return {window_data{
        .form = window_form::range,
        .lo = bound_data{
            .form = bound_form::lit, .lit = lo, .field = name_text{}},
        .hi = bound_data{
            .form = bound_form::lit, .lit = hi, .field = name_text{}},
    }};
}

/// `within(lo, ref(coord))` — a dependent hi bound (target row's u64).
template<class Coordinate>
consteval auto within(std::uint64_t lo, ref_bound<Coordinate>)
    -> capacity_window<Coordinate> {
    return {window_data{
        .form = window_form::range,
        .lo = bound_data{
            .form = bound_form::lit, .lit = lo, .field = name_text{}},
        .hi = bound_data{.form = bound_form::field,
            .lit = 0,
            .field = Coordinate::field_name},
    }};
}

/// `within(lo, duration(coord))` — a dependent hi bound (target
/// interval's measure).
template<class Coordinate>
consteval auto within(std::uint64_t lo, duration_measure<Coordinate>)
    -> capacity_window<Coordinate> {
    return {window_data{
        .form = window_form::range,
        .lo = bound_data{
            .form = bound_form::lit, .lit = lo, .field = name_text{}},
        .hi = bound_data{.form = bound_form::duration_field,
            .lit = 0,
            .field = Coordinate::field_name},
    }};
}

/// A stored capacity law value: target, weight, window, source (the
/// operator read order, C2). The window's numeric payload is the one
/// value-borne datum of the statement algebra.
template<class Target, class Weight, class Source>
struct capacity_law {
    using target_face = Target;
    using source_face = Source;
    using weight_type = Weight;

    window_data window;
};

} // namespace bdb

namespace bdb::detail {

template<class T>
inline constexpr bool is_capacity_v = false;

template<class Target, class Weight, class Source>
inline constexpr bool is_capacity_v<capacity_law<Target, Weight, Source>> =
    true;

template<class T>
inline constexpr bool is_containment_v = false;

template<class Source, class Target, bool B>
inline constexpr bool is_containment_v<containment_law<Source, Target, B>> =
    true;

template<class T>
inline constexpr bool is_statement_v =
    is_key_v<T> || is_containment_v<T> || is_capacity_v<T>;

// Weight field name + form, one reader per case.
struct weight_shape {
    weight_form form;
    name_text field;
};

consteval auto shape_of_weight(unit_weight) -> weight_shape {
    return {weight_form::unit, name_text{}};
}

template<class Coordinate>
consteval auto shape_of_weight(field_weight<Coordinate>) -> weight_shape {
    return {weight_form::field, Coordinate::field_name};
}

template<class Coordinate>
consteval auto shape_of_weight(duration_weight<Coordinate>) -> weight_shape {
    return {weight_form::duration_field, Coordinate::field_name};
}

// The weight coordinate's owner (empty for unit), for the source-roster
// membership check.
consteval auto weight_owner(unit_weight) -> name_text {
    return name_text{};
}

template<class Coordinate>
consteval auto weight_owner(field_weight<Coordinate>) -> name_text {
    return Coordinate::relation_name;
}

template<class Coordinate>
consteval auto weight_owner(duration_weight<Coordinate>) -> name_text {
    return Coordinate::relation_name;
}

template<class Target, class Weight, class Source>
consteval auto capacity_weight_message() -> std::string {
    return "bumbledb capacity(): the weight must read the SOURCE row — "
        "the source face is \""
        + std::string{Source::relation_name.view()}
        + "\" but the weigh() coordinate belongs to another relation";
}

template<class Target, class HiCoordinate>
consteval auto capacity_bound_message() -> std::string {
    return "bumbledb capacity(): a dependent bound resolves against the "
        "TARGET row — the target face is \""
        + std::string{Target::relation_name.view()}
        + "\" but the bound coordinate is \""
        + coordinate_label<HiCoordinate>() + "\"";
}

} // namespace bdb::detail

export namespace bdb {

/// `capacity(target, weigh(...), within(...), source)` — the weighed law.
template<class Target, class Weight, class HiCoordinate, class Source>
consteval auto capacity(Target, Weight,
    capacity_window<HiCoordinate> window, Source)
    -> capacity_law<Target, Weight, Source> {
    static_assert(detail::is_face_v<Target> && detail::is_face_v<Source>,
        "bumbledb capacity(): target and source must be faces — spell "
        "them bdb::on(Relation.field, ...)");
    static_assert(Source::width == Target::width,
        detail::arity_message<Target, Source>("capacity"));
    static_assert(
        std::same_as<Weight, unit_weight>
            || detail::weight_owner(Weight{}) == Source::relation_name,
        detail::capacity_weight_message<Target, Weight, Source>());
    if constexpr (!std::same_as<HiCoordinate, void>) {
        static_assert(
            HiCoordinate::relation_name == Target::relation_name,
            detail::capacity_bound_message<Target, HiCoordinate>());
    }
    return {window.data};
}

/// `capacity(target, within(...), source)` — the unit weight (C4).
template<class Target, class HiCoordinate, class Source>
consteval auto capacity(Target target,
    capacity_window<HiCoordinate> window, Source source)
    -> capacity_law<Target, unit_weight, Source> {
    return capacity(target, unit_weight{}, window, source);
}

} // namespace bdb

namespace bdb::detail {

// ————————————————————————————————————————————————————————————————————
// Reading facades and statements into the flattened tables.
// ————————————————————————————————————————————————————————————————————

/// One facade's flattened relation entry, read off its coordinate types.
template<class Facade>
consteval auto relation_entry() -> relation_data {
    constexpr auto members = std::define_static_array(
        std::meta::nonstatic_data_members_of(
            ^^Facade, std::meta::access_context::current()));
    static_assert(members.size() <= max_relation_fields,
        "bumbledb schema: a relation exceeds max_relation_fields");

    auto out = relation_data{};
    using FirstCoord = [:std::meta::type_of(members[0]):];
    out.name = FirstCoord::relation_name;
    out.field_count = members.size();
    template for (constexpr auto index : index_array<members.size()>()) {
        using Coord = [:std::meta::type_of(members[index]):];
        out.fields[index] = field_data{
            .name = Coord::field_name,
            .kind = Coord::kind,
            .fixed_len = Coord::fixed_len,
            .fresh = Coord::fresh,
        };
    }
    return out;
}

template<class Facade>
consteval auto facade_relation_name_of() -> name_text {
    constexpr auto members = std::define_static_array(
        std::meta::nonstatic_data_members_of(
            ^^Facade, std::meta::access_context::current()));
    using FirstCoord = [:std::meta::type_of(members[0]):];
    return FirstCoord::relation_name;
}

template<class... Args>
consteval auto relation_count() -> std::size_t {
    return (std::size_t{0} + ... + (is_facade<Args>() ? 1U : 0U));
}

template<class... Args>
consteval auto statement_count() -> std::size_t {
    return (std::size_t{0} + ... + (is_statement_v<Args> ? 1U : 0U));
}

template<class... Args>
consteval auto coord_count() -> std::size_t {
    auto count = std::size_t{0};
    auto const add = [&]<class A>() {
        if constexpr (is_facade_type(^^A)) {
            count += relation_entry<A>().field_count;
        }
    };
    (add.template operator()<Args>(), ...);
    return count;
}

template<class... Args>
consteval auto relation_table()
    -> std::array<relation_data, relation_count<Args...>()> {
    auto out = std::array<relation_data, relation_count<Args...>()>{};
    auto index = std::size_t{0};
    auto const add = [&]<class A>() {
        if constexpr (is_facade_type(^^A)) {
            out[index] = relation_entry<A>();
            ++index;
        }
    };
    (add.template operator()<Args>(), ...);
    return out;
}

/// One face type flattened to side data.
template<class Face>
consteval auto side_of() -> side_data {
    auto out = side_data{};
    out.relation = Face::relation_name;
    out.width = Face::width;
    for (auto position = std::size_t{0}; position != Face::width;
        ++position) {
        out.fields[position] = Face::projection[position];
    }
    return out;
}

/// One statement type flattened (the capacity window's numeric payload is
/// value-borne and filled by schema() from the argument value).
template<class Statement>
consteval auto statement_shape() -> statement_data {
    auto out = statement_data{};
    if constexpr (is_key_v<Statement>) {
        out.form = statement_form::key;
        out.source.relation = Statement::relation_name;
        out.source.width = Statement::width;
        for (auto position = std::size_t{0};
            position != Statement::width; ++position) {
            out.source.fields[position] =
                Statement::projection[position];
        }
    } else if constexpr (is_containment_v<Statement>) {
        out.form = statement_form::containment;
        out.source = side_of<typename Statement::source_face>();
        out.target = side_of<typename Statement::target_face>();
        out.bidirectional = Statement::bidirectional;
    } else {
        out.form = statement_form::capacity;
        out.target = side_of<typename Statement::target_face>();
        out.source = side_of<typename Statement::source_face>();
        auto const weight =
            shape_of_weight(typename Statement::weight_type{});
        out.weight = weight.form;
        out.weight_field = weight.field;
    }
    return out;
}

template<class... Args>
consteval auto statement_shapes()
    -> std::array<statement_data, statement_count<Args...>()> {
    auto out = std::array<statement_data, statement_count<Args...>()>{};
    auto index = std::size_t{0};
    auto const add = [&]<class A>() {
        if constexpr (is_statement_v<A>) {
            out[index] = statement_shape<A>();
            ++index;
        }
    };
    (add.template operator()<Args>(), ...);
    return out;
}

// ————————————————————————————————————————————————————————————————————
// The class laws (lowering.md §3): union-find over the projected paired
// faces, the one-generator wall, generator-first naming.
// ————————————————————————————————————————————————————————————————————

inline constexpr std::size_t no_index = ~std::size_t{0};

/// The analysis verdict schema()'s static_asserts read. Total: every
/// check is computed defensively so any single failure produces exactly
/// its own diagnostic.
template<std::size_t CoordCount>
struct law_verdict {
    bool members_known = true;
    std::size_t unknown_statement = 0;
    coord_ref unknown_coordinate{};
    bool relation_missing = false;

    bool lawful = true;
    coord_ref generator_a{};
    coord_ref generator_b{};
    std::size_t wall_statement = 0;

    bool no_restated_implied_key = true;
    std::size_t restated_statement = 0;
    coord_ref restated_fresh{};

    bool no_duplicate_key = true;
    std::size_t duplicate_statement = 0;

    std::array<class_entry, CoordCount> classes{};
};

/// The flat coordinate roster of a relation table.
template<std::size_t CoordCount, std::size_t RelationCount>
consteval auto coord_roster(
    std::array<relation_data, RelationCount> const& relations)
    -> std::array<coord_ref, CoordCount> {
    auto out = std::array<coord_ref, CoordCount>{};
    auto index = std::size_t{0};
    for (auto const& relation : relations) {
        for (auto field = std::size_t{0}; field != relation.field_count;
            ++field) {
            out[index] = coord_ref{
                .relation = relation.name,
                .field = relation.fields[field].name,
            };
            ++index;
        }
    }
    return out;
}

/// The whole §3 computation over the flattened tables.
template<std::size_t CoordCount, std::size_t RelationCount,
    std::size_t StatementCount>
consteval auto analyze(
    std::array<relation_data, RelationCount> const& relations,
    std::array<statement_data, StatementCount> const& statements)
    -> law_verdict<CoordCount> {
    auto verdict = law_verdict<CoordCount>{};
    auto const coords = coord_roster<CoordCount>(relations);

    auto fresh = std::array<bool, CoordCount>{};
    {
        auto index = std::size_t{0};
        for (auto const& relation : relations) {
            for (auto field = std::size_t{0};
                field != relation.field_count; ++field) {
                fresh[index] = relation.fields[field].fresh;
                ++index;
            }
        }
    }

    auto const index_of = [&](name_text relation,
                              name_text field) -> std::size_t {
        for (auto index = std::size_t{0}; index != CoordCount; ++index) {
            if (coords[index].relation == relation
                && coords[index].field == field) {
                return index;
            }
        }
        return no_index;
    };
    auto const relation_known = [&](name_text relation) -> bool {
        for (auto const& entry : relations) {
            if (entry.name == relation) {
                return true;
            }
        }
        return false;
    };

    // Union-find with one generator slot per root (the wall's witness).
    auto parent = std::array<std::size_t, CoordCount>{};
    auto generator = std::array<std::size_t, CoordCount>{};
    for (auto index = std::size_t{0}; index != CoordCount; ++index) {
        parent[index] = index;
        generator[index] = fresh[index] ? index : no_index;
    }
    auto const find = [&](std::size_t at) -> std::size_t {
        while (parent[at] != at) {
            at = parent[at];
        }
        return at;
    };

    auto paired = std::array<bool, CoordCount>{};

    auto const visit_coordinate = [&](std::size_t statement,
                                      name_text relation,
                                      name_text field) -> std::size_t {
        auto const at = index_of(relation, field);
        if (at == no_index && verdict.members_known) {
            verdict.members_known = false;
            verdict.unknown_statement = statement;
            verdict.unknown_coordinate =
                coord_ref{.relation = relation, .field = field};
            verdict.relation_missing = !relation_known(relation);
        }
        return at;
    };

    for (auto statement = std::size_t{0}; statement != StatementCount;
        ++statement) {
        auto const& data = statements[statement];
        if (data.form == statement_form::key) {
            for (auto position = std::size_t{0};
                position != data.source.width; ++position) {
                visit_coordinate(statement, data.source.relation,
                    data.source.fields[position]);
            }
            continue; // an FD pairs nothing (lowering.md §3.3)
        }
        for (auto position = std::size_t{0};
            position != data.source.width; ++position) {
            auto const a = visit_coordinate(statement,
                data.source.relation, data.source.fields[position]);
            auto const b = visit_coordinate(statement,
                data.target.relation, data.target.fields[position]);
            if (a == no_index || b == no_index) {
                continue;
            }
            paired[a] = true;
            paired[b] = true;
            auto const root_a = find(a);
            auto const root_b = find(b);
            if (root_a == root_b) {
                continue;
            }
            parent[root_b] = root_a;
            if (generator[root_a] != no_index
                && generator[root_b] != no_index) {
                if (verdict.lawful) {
                    verdict.lawful = false;
                    verdict.generator_a = coords[generator[root_a]];
                    verdict.generator_b = coords[generator[root_b]];
                    verdict.wall_statement = statement;
                }
            } else if (generator[root_b] != no_index) {
                generator[root_a] = generator[root_b];
            }
        }
    }

    // Naming: generator-first, else the least member coordinate in
    // relation-declaration × field-declaration order (§3.5).
    auto class_name = std::array<std::size_t, CoordCount>{};
    for (auto index = std::size_t{0}; index != CoordCount; ++index) {
        class_name[index] = no_index;
    }
    for (auto index = std::size_t{0}; index != CoordCount; ++index) {
        auto const root = find(index);
        if (class_name[root] == no_index) {
            class_name[root] = generator[root] != no_index
                ? generator[root]
                : index;
        }
    }
    for (auto index = std::size_t{0}; index != CoordCount; ++index) {
        auto const classed = fresh[index] || paired[index];
        verdict.classes[index] = class_entry{
            .coordinate = coords[index],
            .classed = classed,
            .class_name = classed ? coords[class_name[find(index)]]
                                  : coord_ref{},
        };
    }

    // Re-stating an implied key doubles it and moves the fingerprint —
    // reject at construction, like TS (lowering.md §7.1). Also reject an
    // exact duplicate declared key.
    for (auto statement = std::size_t{0}; statement != StatementCount;
        ++statement) {
        auto const& data = statements[statement];
        if (data.form != statement_form::key) {
            continue;
        }
        if (data.source.width == 1) {
            auto const at = index_of(
                data.source.relation, data.source.fields[0]);
            if (at != no_index && fresh[at]
                && verdict.no_restated_implied_key) {
                verdict.no_restated_implied_key = false;
                verdict.restated_statement = statement;
                verdict.restated_fresh = coords[at];
            }
        }
        for (auto other = std::size_t{0}; other != statement; ++other) {
            auto const& prior = statements[other];
            if (prior.form != statement_form::key
                || !(prior.source.relation == data.source.relation)
                || prior.source.width != data.source.width) {
                continue;
            }
            auto equal = true;
            for (auto position = std::size_t{0};
                position != data.source.width; ++position) {
                if (!(prior.source.fields[position]
                        == data.source.fields[position])) {
                    equal = false;
                }
            }
            if (equal && verdict.no_duplicate_key) {
                verdict.no_duplicate_key = false;
                verdict.duplicate_statement = statement;
            }
        }
    }

    return verdict;
}

template<class... Args>
consteval auto analyze_schema()
    -> law_verdict<coord_count<Args...>()> {
    return analyze<coord_count<Args...>()>(
        relation_table<Args...>(), statement_shapes<Args...>());
}

// ————————————————————————————————————————————————————————————————————
// Diagnostics (§34: semantic coordinates, never template internals).
// ————————————————————————————————————————————————————————————————————

consteval auto schema_subject(std::string_view name) -> std::string {
    return std::string{"bumbledb schema \""} + std::string{name} + "\"";
}

/// Renders one flattened statement for the wall diagnostic.
consteval auto render_statement(statement_data const& data) -> std::string {
    auto const render_side = [](side_data const& side) -> std::string {
        auto out = std::string{"on("};
        for (auto position = std::size_t{0}; position != side.width;
            ++position) {
            if (position != 0) {
                out += ", ";
            }
            out += label(side.relation, side.fields[position]);
        }
        return out + ")";
    };
    if (data.form == statement_form::key) {
        auto out = std::string{"key("};
        for (auto position = std::size_t{0};
            position != data.source.width; ++position) {
            if (position != 0) {
                out += ", ";
            }
            out += label(data.source.relation,
                data.source.fields[position]);
        }
        return out + ")";
    }
    if (data.form == statement_form::containment) {
        auto const constructor =
            data.bidirectional ? "mirrors(" : "contained(";
        return constructor + render_side(data.source) + ", "
            + render_side(data.target) + ")";
    }
    return "capacity(" + render_side(data.target) + ", ..., "
        + render_side(data.source) + ")";
}

template<class... Args>
consteval auto membership_message(std::string_view name) -> std::string {
    auto const verdict = analyze_schema<Args...>();
    auto const coordinate = quoted(verdict.unknown_coordinate.relation,
        verdict.unknown_coordinate.field);
    auto out = schema_subject(name) + ": statement "
        + render_count(verdict.unknown_statement)
        + " references coordinate " + coordinate;
    if (verdict.relation_missing) {
        out += " but relation \""
            + std::string{verdict.unknown_coordinate.relation.view()}
            + "\" is not a member of the schema";
    } else {
        out += " but relation \""
            + std::string{verdict.unknown_coordinate.relation.view()}
            + "\" declares no such field";
    }
    return out;
}

template<class... Args>
consteval auto wall_message(std::string_view name) -> std::string {
    auto const verdict = analyze_schema<Args...>();
    auto const statements = statement_shapes<Args...>();
    return schema_subject(name)
        + ": the statements unify two generators into one class — "
        + quoted(verdict.generator_a.relation, verdict.generator_a.field)
        + " and "
        + quoted(verdict.generator_b.relation, verdict.generator_b.field)
        + " (two mints cannot share a carrier) — "
        + render_statement(statements[verdict.wall_statement]);
}

template<class... Args>
consteval auto restated_key_message(std::string_view name) -> std::string {
    auto const verdict = analyze_schema<Args...>();
    return schema_subject(name) + ": "
        + render_statement(
            statement_shapes<Args...>()[verdict.restated_statement])
        + " restates the fresh-implied key of "
        + quoted(verdict.restated_fresh.relation,
            verdict.restated_fresh.field)
        + " — the engine materializes implied keys; restating one doubles "
          "it and moves the fingerprint";
}

template<class... Args>
consteval auto duplicate_key_message(std::string_view name) -> std::string {
    auto const verdict = analyze_schema<Args...>();
    return schema_subject(name) + ": "
        + render_statement(
            statement_shapes<Args...>()[verdict.duplicate_statement])
        + " duplicates an earlier declared key";
}

/// Whether relations precede statements (the pinned argument shape).
template<class... Args>
consteval auto relations_lead() -> bool {
    auto seen_statement = false;
    auto ordered = true;
    auto const step = [&]<class A>() {
        if constexpr (is_statement_v<A>) {
            seen_statement = true;
        } else {
            if (seen_statement) {
                ordered = false;
            }
        }
    };
    (step.template operator()<Args>(), ...);
    return ordered;
}

template<class... Args>
consteval auto args_recognized() -> bool {
    return ((is_facade<Args>() || is_statement_v<Args>) && ...);
}

template<class... Args>
consteval auto relation_names_distinct() -> bool {
    auto const relations = relation_table<Args...>();
    for (auto first = std::size_t{0}; first != relations.size(); ++first) {
        for (auto second = first + 1; second != relations.size();
            ++second) {
            if (relations[first].name == relations[second].name) {
                return false;
            }
        }
    }
    return true;
}

// ————————————————————————————————————————————————————————————————————
// The relations-as-members injection.
// ————————————————————————————————————————————————————————————————————

template<class... Args>
struct schema_relation_types {
    struct Relations;
    consteval {
        auto specs = std::vector<std::meta::info>{};
        [[maybe_unused]] auto const add = [&]<class A>() {
            // The branch must be constexpr: the name reader may only be
            // instantiated for facade types.
            if constexpr (is_facade_type(^^A)) {
                // Skip a duplicate name so the injection stays total and
                // schema()'s static_assert carries the one diagnostic.
                auto const name = facade_relation_name_of<A>();
                for (auto const spec : specs) {
                    if (std::meta::identifier_of(spec) == name.view()) {
                        return;
                    }
                }
                specs.push_back(std::meta::data_member_spec(
                    ^^A, {.name = spec_name(name.view())}));
            }
        };
        (add.template operator()<Args>(), ...);
        std::meta::define_aggregate(^^Relations, specs);
    }
};

// The fresh-pattern injection (§26's primary lane): one member per
// fresh-marked coordinate of the facade.
template<class Facade>
struct fresh_pattern_types {
    struct Pattern;
    consteval {
        auto specs = std::vector<std::meta::info>{};
        for (auto const member : std::meta::nonstatic_data_members_of(
                 ^^Facade, std::meta::access_context::current())) {
            auto const args = std::meta::template_arguments_of(
                std::meta::dealias(std::meta::type_of(member)));
            if (std::meta::extract<bool>(args[5])) {
                specs.push_back(std::meta::data_member_spec(args[0],
                    {.name = spec_name(
                         std::meta::extract<name_text>(args[2]).view())}));
            }
        }
        std::meta::define_aggregate(^^Pattern, specs);
    }
};

} // namespace bdb::detail

export namespace bdb {

/// The fresh-field primary read pattern of a facade (§26):
/// `db.get(Service, {.id = id})`.
template<class Facade>
using fresh_pattern_of =
    typename detail::fresh_pattern_types<Facade>::Pattern;

/// The facade's fresh-field count (constrains the primary get lane).
template<class Facade>
consteval auto fresh_field_count() -> std::size_t {
    auto const entry = detail::relation_entry<Facade>();
    auto count = std::size_t{0};
    for (auto field = std::size_t{0}; field != entry.field_count;
        ++field) {
        if (entry.fields[field].fresh) {
            ++count;
        }
    }
    return count;
}

// ————————————————————————————————————————————————————————————————————
// The schema value.
// ————————————————————————————————————————————————————————————————————

/// A whole theory as one literal value (TODO_CPP §9–§10): relations as
/// members (argument order = declaration order = RelationId mint), the
/// flattened SchemaSpec data (DECLARED statements only, written order),
/// and the law-computed class map with a consteval lookup. Structural —
/// a schema travels as a template argument.
template<fixed_string Name, class... Args>
struct schema_value {
    static constexpr std::size_t relation_count =
        detail::relation_count<Args...>();
    static constexpr std::size_t statement_count =
        detail::statement_count<Args...>();
    static constexpr std::size_t coordinate_count =
        detail::coord_count<Args...>();

    /// The schema's name (diagnostics only; never on the wire).
    name_text schema_name;

    /// The schema's name at the TYPE tier (query-layer diagnostics run in
    /// contexts where only template arguments are constant-visible).
    static constexpr auto declared_name = Name;

    /// The relation facades, one member per relation, named identically —
    /// `Uptime.relations.Service.id` (the later query surface).
    typename detail::schema_relation_types<Args...>::Relations relations;

    /// Relations/fields in declaration order — the wire lane's source.
    std::array<relation_data, relation_count> relation_table;

    /// DECLARED statements only, written order (lowering.md §2.1/§7.1).
    std::array<statement_data, statement_count> statements;

    /// The class map: one entry per coordinate in relation-declaration ×
    /// field-declaration order (lowering.md §3).
    std::array<class_entry, coordinate_count> classes;

    /// Type-derivable twins of the value tables: the query elaborator
    /// (bumbledb.meta.query) recomputes the flattened relation table and
    /// the class map from the schema TYPE during rule elaboration — the
    /// static_assert walls there can only read template arguments, never
    /// a schema VALUE. Deterministic: schema() fills the members from
    /// exactly these computations.
    static consteval auto member_relation_table()
        -> std::array<relation_data, relation_count> {
        return detail::relation_table<Args...>();
    }

    static consteval auto member_class_map()
        -> std::array<class_entry, coordinate_count> {
        return detail::analyze_schema<Args...>().classes;
    }

    /// The consteval class lookup: a coordinate's law-computed class
    /// identity (nullopt = bare). The class NAME is the returned
    /// coordinate rendered "Relation.field".
    template<class Coordinate>
    [[nodiscard]] consteval auto class_of(Coordinate) const
        -> std::optional<coord_ref> {
        static_assert(detail::is_coordinate_v<Coordinate>,
            "bumbledb schema::class_of(): the argument must be a relation "
            "coordinate (Relation.field)");
        for (auto const& entry : classes) {
            if (entry.coordinate.relation == Coordinate::relation_name
                && entry.coordinate.field == Coordinate::field_name) {
                if (!entry.classed) {
                    return std::nullopt;
                }
                return entry.class_name;
            }
        }
        return std::nullopt;
    }
};

/// The consteval schema elaborator (TODO_CPP §9–§10): relations first (in
/// declaration order), then statements (in written order). Runs the class
/// laws and every structurally decidable §34 wall; hands NOTHING semantic
/// to the wire beyond names — the engine's SchemaSpec::descriptor()
/// remains the authority.
template<fixed_string Name, class... Args>
consteval auto schema(Args const&... args) -> schema_value<Name, Args...> {
    static_assert(detail::args_recognized<Args...>(),
        "bumbledb schema(): every argument must be a relation facade "
        "(bdb::relation<...>) or a statement value (bdb::key / "
        "bdb::contained / bdb::mirrors / bdb::capacity)");
    static_assert(detail::relation_count<Args...>() > 0,
        "bumbledb schema(): a schema declares at least one relation");
    static_assert(detail::relations_lead<Args...>(),
        "bumbledb schema(): relations precede statements — declare every "
        "member facade before the first statement");
    static_assert(detail::relation_names_distinct<Args...>(),
        "bumbledb schema(): two member relations share one name");

    static_assert(detail::analyze_schema<Args...>().members_known,
        detail::membership_message<Args...>(Name.view()));
    static_assert(detail::analyze_schema<Args...>().lawful,
        detail::wall_message<Args...>(Name.view()));
    static_assert(
        detail::analyze_schema<Args...>().no_restated_implied_key,
        detail::restated_key_message<Args...>(Name.view()));
    static_assert(detail::analyze_schema<Args...>().no_duplicate_key,
        detail::duplicate_key_message<Args...>(Name.view()));

    auto out = schema_value<Name, Args...>{
        .schema_name = detail::to_name_text(Name.view()),
        .relations = {},
        .relation_table = detail::relation_table<Args...>(),
        .statements = detail::statement_shapes<Args...>(),
        .classes = detail::analyze_schema<Args...>().classes,
    };

    // The one value-borne payload: capacity windows (their numeric
    // bounds are argument VALUES, not types).
    auto index = std::size_t{0};
    auto const fill = [&]<class A>(A const& argument) {
        if constexpr (detail::is_statement_v<A>) {
            if constexpr (detail::is_capacity_v<A>) {
                out.statements[index].window = argument.window;
            }
            ++index;
        }
    };
    (fill(args), ...);
    return out;
}

/// The schema concept the runtime lane (bumbledb.db) admits: any literal
/// value carrying the flattened tables (structural, never nominal).
template<class S>
concept Theory = requires(S const& theory) {
    requires std::same_as<std::ranges::range_value_t<
                              decltype(theory.relation_table)>,
        relation_data>;
    requires std::same_as<
        std::ranges::range_value_t<decltype(theory.statements)>,
        statement_data>;
    requires std::same_as<
        std::ranges::range_value_t<decltype(theory.classes)>,
        class_entry>;
};

} // namespace bdb
