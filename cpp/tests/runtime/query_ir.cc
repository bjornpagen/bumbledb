import std;
import bumbledb;

struct ServiceRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct OutageRow {
	std::uint64_t service;
	bdb::interval<std::int64_t> window;
};

inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;
inline constexpr auto Outage = bdb::relation<"Outage", OutageRow>;

inline constexpr auto Uptime = bdb::schema<"Uptime">(Service, Outage,

                                                     bdb::contained(bdb::on(Outage.service), bdb::on(Service.id)),

                                                     bdb::key(Outage.service, Outage.window));

inline constexpr auto DownAt = bdb::query(Uptime).rule([](auto r) consteval {
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

inline constexpr auto Overlapping = bdb::query(Uptime).rule([](auto r) consteval {
	auto vars = r.vars(Outage);
	return r
	    .match(Outage,
	           {
	               .service = vars.service,
	               .window = vars.window,
	           })
	    .where(bdb::allen_in(vars.window, bdb::allen::intersects, bdb::param<"incident">()))
	    .find({
	        .service = vars.service,
	        .window = vars.window,
	    });
});

inline constexpr auto Downtime = bdb::query(Uptime).rule([](auto r) consteval {
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

inline constexpr auto NamedDownAt = bdb::query(Uptime).rule([](auto r) consteval {
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

inline constexpr auto LongOutages = bdb::query(Uptime).rule([](auto r) consteval {
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

[[nodiscard]] consteval auto text_is(bdb::name_text name, std::string_view want) -> bool {
	return name.view() == want;
}

}

static_assert(DownAt.ir.rule_count == 1);
static_assert(DownAt.ir.head_count == 1);
static_assert(DownAt.ir.param_count == 1);

static_assert(text_is(DownAt.ir.params[0].name, "t"));
static_assert(DownAt.ir.params[0].shape == bdb::param_shape::value);
static_assert(DownAt.ir.params[0].domain == bdb::field_class{bdb::value_kind::i64, 0});
static_assert(DownAt.ir.params[0].point);

static_assert(DownAt.ir.rules[0].atom_count == 1);
static_assert(DownAt.ir.rules[0].atoms[0].relation == 1);
static_assert(DownAt.ir.rules[0].atoms[0].binding_count == 2);
static_assert(DownAt.ir.rules[0].atoms[0].bindings[0].field == 0);
static_assert(DownAt.ir.rules[0].atoms[0].bindings[0].term.form == bdb::query_term_form::variable);
static_assert(DownAt.ir.rules[0].atoms[0].bindings[0].term.var == 0);
static_assert(DownAt.ir.rules[0].atoms[0].bindings[1].field == 1);
static_assert(DownAt.ir.rules[0].atoms[0].bindings[1].term.var == 1);

static_assert(DownAt.ir.rules[0].condition_count == 1);
static_assert(DownAt.ir.rules[0].conditions[0].op == bdb::query_cmp::point_in);
static_assert(DownAt.ir.rules[0].conditions[0].lhs.form == bdb::query_term_form::variable);
static_assert(DownAt.ir.rules[0].conditions[0].lhs.var == 1);
static_assert(DownAt.ir.rules[0].conditions[0].rhs.form == bdb::query_term_form::param);
static_assert(DownAt.ir.rules[0].conditions[0].rhs.param == 0);

static_assert(DownAt.ir.rules[0].find_count == 1);
static_assert(DownAt.ir.rules[0].finds[0].form == bdb::find_form::variable);
static_assert(DownAt.ir.rules[0].finds[0].over == 0);
static_assert(text_is(DownAt.ir.head[0].name, "service"));
static_assert(DownAt.ir.head[0].answer == bdb::field_class{bdb::value_kind::u64, 0});

static_assert(std::same_as<decltype(std::declval<bdb::row_of<DownAt>>().service), std::uint64_t>);
static_assert(std::same_as<decltype(std::declval<bdb::params_of<DownAt>>().t), std::int64_t>);

static_assert(Overlapping.ir.param_count == 1);
static_assert(text_is(Overlapping.ir.params[0].name, "incident"));
static_assert(Overlapping.ir.params[0].domain == bdb::field_class{bdb::value_kind::interval_i64, 0});
static_assert(!Overlapping.ir.params[0].point);
static_assert(Overlapping.ir.rules[0].condition_count == 1);
static_assert(Overlapping.ir.rules[0].conditions[0].op == bdb::query_cmp::allen);
static_assert(Overlapping.ir.rules[0].conditions[0].mask == bdb::allen::intersects.bits());
static_assert(Overlapping.ir.rules[0].conditions[0].lhs.var == 1);
static_assert(Overlapping.ir.rules[0].conditions[0].rhs.form == bdb::query_term_form::param);
static_assert(Overlapping.ir.head_count == 2);
static_assert(text_is(Overlapping.ir.head[1].name, "window"));
static_assert(Overlapping.ir.head[1].answer == bdb::field_class{bdb::value_kind::interval_i64, 0});
static_assert(std::same_as<decltype(std::declval<bdb::row_of<Overlapping>>().window), bdb::interval<std::int64_t>>);
static_assert(std::same_as<decltype(std::declval<bdb::params_of<Overlapping>>().incident), bdb::interval<std::int64_t>>);

static_assert(Downtime.ir.param_count == 0);
static_assert(Downtime.ir.head_count == 2);
static_assert(Downtime.ir.rules[0].find_count == 2);
static_assert(Downtime.ir.rules[0].finds[1].form == bdb::find_form::aggregate_measure);
static_assert(Downtime.ir.rules[0].finds[1].op == bdb::fold_form::sum);
static_assert(Downtime.ir.rules[0].finds[1].over == 1);
static_assert(text_is(Downtime.ir.head[1].name, "downtime"));
static_assert(Downtime.ir.head[1].answer == bdb::field_class{bdb::value_kind::u64, 0});
static_assert(std::same_as<decltype(std::declval<bdb::row_of<Downtime>>().downtime), std::uint64_t>);

static_assert(LongOutages.ir.param_count == 0);
static_assert(LongOutages.ir.rules[0].condition_count == 1);
static_assert(LongOutages.ir.rules[0].conditions[0].op == bdb::query_cmp::ge);
static_assert(LongOutages.ir.rules[0].conditions[0].lhs.form == bdb::query_term_form::measure);
static_assert(LongOutages.ir.rules[0].conditions[0].lhs.var == 1);
static_assert(LongOutages.ir.rules[0].conditions[0].rhs.form == bdb::query_term_form::literal);
static_assert(LongOutages.ir.rules[0].conditions[0].rhs.literal.kind == bdb::value_kind::u64);
static_assert(LongOutages.ir.rules[0].conditions[0].rhs.literal.u64 == 100);

static_assert(NamedDownAt.ir.rules[0].atom_count == 2);
static_assert(NamedDownAt.ir.rules[0].atoms[1].relation == 0);
static_assert(NamedDownAt.ir.rules[0].atoms[1].binding_count == 2);
static_assert(NamedDownAt.ir.rules[0].atoms[1].bindings[0].field == 0);
static_assert(NamedDownAt.ir.rules[0].atoms[1].bindings[0].term.var == 0);
static_assert(NamedDownAt.ir.rules[0].atoms[1].bindings[1].field == 1);
static_assert(NamedDownAt.ir.rules[0].atoms[1].bindings[1].term.var == 2);
static_assert(NamedDownAt.ir.rules[0].finds[0].over == 2);
static_assert(NamedDownAt.ir.head[0].answer == bdb::field_class{bdb::value_kind::string, 0});
static_assert(std::same_as<decltype(std::declval<bdb::row_of<NamedDownAt>>().name), std::string_view>);

auto main() -> int {
	std::println("pass: recipe-1 query IR lowers to the lower.ts shape "
	             "(vars/atoms/conditions/params/head, all pinned)");
	std::println("pass: row_of/params_of synthesize named, exactly-typed "
	             "products");
	return 0;
}
