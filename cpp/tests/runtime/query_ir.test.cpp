// The query elaborator's lowering proofs (TODO_CPP §11–§12, §21;
// lowering.md §4.2): recipe 1's three queries lower during constant
// evaluation to exactly the IR ts/src/query/lower.ts builds — dense
// rule-scoped variable ids by first occurrence over the written walk
// (bindings in written order at field ordinals, finds last), pointIn
// stored interval-LEFT, the param registry in first-use order with
// field-anchored domains and point flags, and the one-predicate program
// head. Everything here is a static_assert over the query VALUES; main()
// exists so the proofs run under ctest like every other runtime target.
import std;
import bumbledb.types;
import bumbledb.meta.relation;
import bumbledb.meta.schema;
import bumbledb.meta.query;

// Recipe 1 (ts/COOKBOOK.md §1), spelled through the real elaborators.
struct ServiceRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    std::string name;
};

struct OutageRow {
    std::uint64_t service;
    bdb::interval<std::int64_t> window;
};

inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;
inline constexpr auto Outage = bdb::relation<"Outage", OutageRow>;

inline constexpr auto Uptime = bdb::schema<"Uptime">(
    Service,
    Outage,

    bdb::contained(
        bdb::on(Outage.service),
        bdb::on(Service.id)
    ),

    bdb::key(
        Outage.service,
        Outage.window
    )
);

// down at instant t (COOKBOOK.md:130-136).
inline constexpr auto DownAt =
    bdb::query(Uptime).rule([](auto r) consteval {
        auto vars = r.vars(Outage);
        return r
            .match(Outage,
                {
                    .service = vars.service,
                    .window = vars.window,
                })
            .where(bdb::point_in(bdb::param<"t">(), vars.window))
            .find({
                .service = vars.service,
            });
    });

// overlapping an incident window (COOKBOOK.md:138-144).
inline constexpr auto Overlapping =
    bdb::query(Uptime).rule([](auto r) consteval {
        auto vars = r.vars(Outage);
        return r
            .match(Outage,
                {
                    .service = vars.service,
                    .window = vars.window,
                })
            .where(bdb::allen_in(vars.window, bdb::allen::intersects,
                bdb::param<"incident">()))
            .find({
                .service = vars.service,
                .window = vars.window,
            });
    });

// total downtime per service (COOKBOOK.md:146-149).
inline constexpr auto Downtime =
    bdb::query(Uptime).rule([](auto r) consteval {
        auto vars = r.vars(Outage);
        return r
            .match(Outage,
                {
                    .service = vars.service,
                    .window = vars.window,
                })
            .find(
                {
                    .service = vars.service,
                },
                bdb::sum<"downtime">(r.duration(vars.window)));
    });

// A two-relation join: the classed variable reuse the law classes ADMIT —
// Outage.service and Service.id share the class "Service.id".
inline constexpr auto NamedDownAt =
    bdb::query(Uptime).rule([](auto r) consteval {
        auto outage = r.vars(Outage);
        auto service = r.vars(Service);
        return r
            .match(Outage,
                {
                    .service = outage.service,
                    .window = outage.window,
                })
            .match(Service,
                {
                    .id = outage.service,
                    .name = service.name,
                })
            .where(bdb::point_in(bdb::param<"t">(), outage.window))
            .find({
                .name = service.name,
            });
    });

// A scalar order comparison over the measure (`ir::Term::Measure` is
// legal as one side of an order comparison): outages at least 100 long.
inline constexpr auto LongOutages =
    bdb::query(Uptime).rule([](auto r) consteval {
        auto vars = r.vars(Outage);
        return r
            .match(Outage,
                {
                    .service = vars.service,
                    .window = vars.window,
                })
            .where(bdb::ge(r.duration(vars.window), std::uint64_t{100}))
            .find({
                .service = vars.service,
            });
    });

namespace {

consteval auto text_is(bdb::name_text name, std::string_view want) -> bool {
    return name.view() == want;
}

} // namespace

// --- DownAt: the lower.ts shape, member for member --------------------------

static_assert(DownAt.ir.rule_count == 1);
static_assert(DownAt.ir.head_count == 1);
static_assert(DownAt.ir.param_count == 1);

// Param registry: first use mints ParamId 0; the pointIn use anchors the
// interval element domain (i64) and marks the point flag (TODO_CPP §21).
static_assert(text_is(DownAt.ir.params[0].name, "t"));
static_assert(DownAt.ir.params[0].shape == bdb::param_shape::value);
static_assert(
    DownAt.ir.params[0].domain
    == bdb::field_class{bdb::value_kind::i64, 0});
static_assert(DownAt.ir.params[0].point);

// One EDB atom over Outage (declaration ordinal 1), bindings in written
// order at field ordinals; vars minted by first occurrence: service=0,
// window=1 (lowering.md §4.2).
static_assert(DownAt.ir.rules[0].atom_count == 1);
static_assert(DownAt.ir.rules[0].atoms[0].relation == 1);
static_assert(DownAt.ir.rules[0].atoms[0].binding_count == 2);
static_assert(DownAt.ir.rules[0].atoms[0].bindings[0].field == 0);
static_assert(DownAt.ir.rules[0].atoms[0].bindings[0].term.form
    == bdb::query_term_form::variable);
static_assert(DownAt.ir.rules[0].atoms[0].bindings[0].term.var == 0);
static_assert(DownAt.ir.rules[0].atoms[0].bindings[1].field == 1);
static_assert(DownAt.ir.rules[0].atoms[0].bindings[1].term.var == 1);

// pointIn lowers interval-LEFT, point-RIGHT whatever the surface order
// (ts/src/query/atom.ts:432-435).
static_assert(DownAt.ir.rules[0].condition_count == 1);
static_assert(
    DownAt.ir.rules[0].conditions[0].op == bdb::query_cmp::point_in);
static_assert(DownAt.ir.rules[0].conditions[0].lhs.form
    == bdb::query_term_form::variable);
static_assert(DownAt.ir.rules[0].conditions[0].lhs.var == 1);
static_assert(DownAt.ir.rules[0].conditions[0].rhs.form
    == bdb::query_term_form::param);
static_assert(DownAt.ir.rules[0].conditions[0].rhs.param == 0);

// Finds last: the head derives {service: var 0}.
static_assert(DownAt.ir.rules[0].find_count == 1);
static_assert(
    DownAt.ir.rules[0].finds[0].form == bdb::find_form::variable);
static_assert(DownAt.ir.rules[0].finds[0].over == 0);
static_assert(text_is(DownAt.ir.head[0].name, "service"));
static_assert(DownAt.ir.head[0].answer
    == bdb::field_class{bdb::value_kind::u64, 0});

// The synthesized products (TODO_CPP §12, §21): named members, exact
// types — a wrong name or type at execute is a compile error.
static_assert(
    std::same_as<decltype(std::declval<bdb::row_of<DownAt>>().service), std::uint64_t>);
static_assert(
    std::same_as<decltype(std::declval<bdb::params_of<DownAt>>().t), std::int64_t>);

// --- Overlapping: the allen mask + the interval-anchored param --------------

static_assert(Overlapping.ir.param_count == 1);
static_assert(text_is(Overlapping.ir.params[0].name, "incident"));
static_assert(Overlapping.ir.params[0].domain
    == bdb::field_class{bdb::value_kind::interval_i64, 0});
static_assert(!Overlapping.ir.params[0].point);
static_assert(Overlapping.ir.rules[0].condition_count == 1);
static_assert(
    Overlapping.ir.rules[0].conditions[0].op == bdb::query_cmp::allen);
static_assert(Overlapping.ir.rules[0].conditions[0].mask
    == bdb::allen::intersects.bits());
static_assert(Overlapping.ir.rules[0].conditions[0].lhs.var == 1);
static_assert(Overlapping.ir.rules[0].conditions[0].rhs.form
    == bdb::query_term_form::param);
static_assert(Overlapping.ir.head_count == 2);
static_assert(text_is(Overlapping.ir.head[1].name, "window"));
static_assert(Overlapping.ir.head[1].answer
    == bdb::field_class{bdb::value_kind::interval_i64, 0});
static_assert(std::same_as<decltype(std::declval<bdb::row_of<Overlapping>>().window),
    bdb::interval<std::int64_t>>);
static_assert(std::same_as<decltype(std::declval<bdb::params_of<Overlapping>>().incident),
    bdb::interval<std::int64_t>>);

// --- Downtime: the sum(duration) aggregate head ------------------------------

static_assert(Downtime.ir.param_count == 0);
static_assert(Downtime.ir.head_count == 2);
static_assert(Downtime.ir.rules[0].find_count == 2);
static_assert(Downtime.ir.rules[0].finds[1].form
    == bdb::find_form::aggregate_measure);
static_assert(Downtime.ir.rules[0].finds[1].op == bdb::fold_form::sum);
static_assert(Downtime.ir.rules[0].finds[1].over == 1);
static_assert(text_is(Downtime.ir.head[1].name, "downtime"));
static_assert(Downtime.ir.head[1].answer
    == bdb::field_class{bdb::value_kind::u64, 0});
static_assert(
    std::same_as<decltype(std::declval<bdb::row_of<Downtime>>().downtime),
        std::uint64_t>);

// --- LongOutages: the measure under an order comparison ----------------------
// The measure sibling types the literal (u64 — lowering.md §4.2's
// sibling-directed tagging).

static_assert(LongOutages.ir.param_count == 0);
static_assert(LongOutages.ir.rules[0].condition_count == 1);
static_assert(
    LongOutages.ir.rules[0].conditions[0].op == bdb::query_cmp::ge);
static_assert(LongOutages.ir.rules[0].conditions[0].lhs.form
    == bdb::query_term_form::measure);
static_assert(LongOutages.ir.rules[0].conditions[0].lhs.var == 1);
static_assert(LongOutages.ir.rules[0].conditions[0].rhs.form
    == bdb::query_term_form::literal);
static_assert(LongOutages.ir.rules[0].conditions[0].rhs.literal.kind
    == bdb::value_kind::u64);
static_assert(
    LongOutages.ir.rules[0].conditions[0].rhs.literal.u64 == 100);

// --- NamedDownAt: the JOIN — variable reuse across one law class ------------
// Numbering by first occurrence across BOTH atoms: Outage.service = 0,
// Outage.window = 1, then Service.id reuses var 0 (the join), and
// Service.name mints var 2. Finds last: name = var 2.

static_assert(NamedDownAt.ir.rules[0].atom_count == 2);
static_assert(NamedDownAt.ir.rules[0].atoms[1].relation == 0);
static_assert(NamedDownAt.ir.rules[0].atoms[1].binding_count == 2);
static_assert(NamedDownAt.ir.rules[0].atoms[1].bindings[0].field == 0);
static_assert(NamedDownAt.ir.rules[0].atoms[1].bindings[0].term.var == 0);
static_assert(NamedDownAt.ir.rules[0].atoms[1].bindings[1].field == 1);
static_assert(NamedDownAt.ir.rules[0].atoms[1].bindings[1].term.var == 2);
static_assert(NamedDownAt.ir.rules[0].finds[0].over == 2);
static_assert(NamedDownAt.ir.head[0].answer
    == bdb::field_class{bdb::value_kind::string, 0});
static_assert(
    std::same_as<decltype(std::declval<bdb::row_of<NamedDownAt>>().name),
        std::string_view>);

auto main() -> int {
    std::println("pass: recipe-1 query IR lowers to the lower.ts shape "
                 "(vars/atoms/conditions/params/head, all pinned)");
    std::println("pass: row_of/params_of synthesize named, exactly-typed "
                 "products");
    return 0;
}
