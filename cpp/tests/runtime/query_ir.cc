import std;
import bumbledb;
import bumbledb_foreign;

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

inline constexpr auto LongOrShort = bdb::query(Uptime).rule([](auto r) consteval {
	auto vars = r.vars(Outage);
	return r
	    .match(Outage,
	           {
	               .service = vars.service,
	               .window = vars.window,
	           })
	    .where(r.Or(bdb::ge(r.duration(vars.window), std::uint64_t{100}), bdb::lt(r.duration(vars.window), std::uint64_t{80})))
	    .find({
	        .service = vars.service,
	    });
});

inline constexpr auto WindowLen = bdb::query(Uptime).rule([](auto r) consteval {
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
	        bdb::as<"len">(r.duration(vars.window)));
});

namespace {

[[nodiscard]] consteval auto text_is(bdb::name_text name, std::string_view want) -> bool {
	return name.view() == want;
}

}

static_assert(DownAt.rules.size() == 1);
static_assert(DownAt.head_count == 1);
static_assert(DownAt.param_count == 1);

static_assert(text_is(DownAt.params[0].name, "t"));
static_assert(DownAt.params[0].form == bdb::param_form::point);
static_assert(DownAt.params[0].domain == bdb::field_class{bdb::value_kind::i64, 0});

static_assert(DownAt.rules[0].atom_count == 1);
static_assert(DownAt.rules[0].atoms[0].source == bdb::atom_source::edb);
static_assert(DownAt.rules[0].atoms[0].id == 1);
static_assert(DownAt.rules[0].atoms[0].binding_count == 2);
static_assert(DownAt.rules[0].atoms[0].bindings[0].field == 0);
static_assert(DownAt.rules[0].atoms[0].bindings[0].term.form == bdb::query_term_form::variable);
static_assert(DownAt.rules[0].atoms[0].bindings[0].term.var == 0);
static_assert(DownAt.rules[0].atoms[0].bindings[1].field == 1);
static_assert(DownAt.rules[0].atoms[0].bindings[1].term.var == 1);

static_assert(DownAt.rules[0].condition_count == 1);
static_assert(DownAt.rules[0].conditions[0].op == bdb::query_cmp::point_in);
static_assert(DownAt.rules[0].conditions[0].lhs.form == bdb::query_term_form::variable);
static_assert(DownAt.rules[0].conditions[0].lhs.var == 1);
static_assert(DownAt.rules[0].conditions[0].rhs.form == bdb::query_term_form::param);
static_assert(DownAt.rules[0].conditions[0].rhs.param == 0);

static_assert(DownAt.rules[0].find_count == 1);
static_assert(DownAt.rules[0].finds[0].form == bdb::find_form::variable);
static_assert(DownAt.rules[0].finds[0].over == 0);
static_assert(text_is(DownAt.head[0].name, "service"));
static_assert(DownAt.head[0].answer == bdb::field_class{bdb::value_kind::u64, 0});

static_assert(std::same_as<decltype(std::declval<bdb::row_of<DownAt>>().service), std::uint64_t>);
static_assert(std::same_as<decltype(std::declval<bdb::params_of<DownAt>>().t), std::int64_t>);

static_assert(Overlapping.param_count == 1);
static_assert(text_is(Overlapping.params[0].name, "incident"));
static_assert(Overlapping.params[0].domain == bdb::field_class{bdb::value_kind::interval_i64, 0});
static_assert(Overlapping.params[0].form == bdb::param_form::value);
static_assert(Overlapping.rules[0].condition_count == 1);
static_assert(Overlapping.rules[0].conditions[0].op == bdb::query_cmp::allen);
static_assert(Overlapping.rules[0].conditions[0].mask == bdb::allen::intersects.bits());
static_assert(Overlapping.rules[0].conditions[0].lhs.var == 1);
static_assert(Overlapping.rules[0].conditions[0].rhs.form == bdb::query_term_form::param);
static_assert(Overlapping.head_count == 2);
static_assert(text_is(Overlapping.head[1].name, "window"));
static_assert(Overlapping.head[1].answer == bdb::field_class{bdb::value_kind::interval_i64, 0});
static_assert(std::same_as<decltype(std::declval<bdb::row_of<Overlapping>>().window), bdb::interval<std::int64_t>>);
static_assert(std::same_as<decltype(std::declval<bdb::params_of<Overlapping>>().incident), bdb::interval<std::int64_t>>);

static_assert(Downtime.param_count == 0);
static_assert(Downtime.head_count == 2);
static_assert(Downtime.rules[0].find_count == 2);
static_assert(Downtime.rules[0].finds[1].form == bdb::find_form::aggregate_measure);
static_assert(Downtime.rules[0].finds[1].op == bdb::fold_form::sum);
static_assert(Downtime.rules[0].finds[1].over == 1);
static_assert(text_is(Downtime.head[1].name, "downtime"));
static_assert(Downtime.head[1].answer == bdb::field_class{bdb::value_kind::u64, 0});
static_assert(std::same_as<decltype(std::declval<bdb::row_of<Downtime>>().downtime), std::uint64_t>);

static_assert(LongOutages.param_count == 0);
static_assert(LongOutages.rules[0].condition_count == 1);
static_assert(LongOutages.rules[0].conditions[0].op == bdb::query_cmp::ge);
static_assert(LongOutages.rules[0].conditions[0].lhs.form == bdb::query_term_form::measure);
static_assert(LongOutages.rules[0].conditions[0].lhs.var == 1);
static_assert(LongOutages.rules[0].conditions[0].rhs.form == bdb::query_term_form::literal);
static_assert(LongOutages.rules[0].conditions[0].rhs.literal.kind == bdb::value_kind::u64);
static_assert(LongOutages.rules[0].conditions[0].rhs.literal.u64 == 100);

static_assert(LongOrShort.rules[0].condition_count == 1);
static_assert(LongOrShort.rules[0].condition_node_count == 3);
static_assert(LongOrShort.rules[0].conditions[0].form == bdb::condition_form::or_node);
static_assert(LongOrShort.rules[0].conditions[0].child_count == 2);
static_assert(LongOrShort.rules[0].conditions[0].child_begin == 1);
static_assert(LongOrShort.rules[0].conditions[1].form == bdb::condition_form::leaf);
static_assert(LongOrShort.rules[0].conditions[1].op == bdb::query_cmp::ge);
static_assert(LongOrShort.rules[0].conditions[2].form == bdb::condition_form::leaf);
static_assert(LongOrShort.rules[0].conditions[2].op == bdb::query_cmp::lt);
static_assert(bdb::foreign::query_of<LongOrShort>.rules[0].conditions[0].kind ==
              static_cast<std::uint32_t>(bdb::foreign::bdb_condition_kind::BDB_CONDITION_KIND_OR));
static_assert(bdb::foreign::query_of<LongOrShort>.rules[0].conditions[0].child_count == 2);
static_assert(bdb::foreign::query_of<LongOrShort>.rules[0].condition_count == 1);

static_assert(WindowLen.head_count == 2);
static_assert(WindowLen.rules[0].find_count == 2);
static_assert(WindowLen.rules[0].finds[1].form == bdb::find_form::measure);
static_assert(WindowLen.rules[0].finds[1].over == 1);
static_assert(text_is(WindowLen.head[1].name, "len"));
static_assert(WindowLen.head[1].answer == bdb::field_class{bdb::value_kind::u64, 0});
static_assert(bdb::foreign::query_of<WindowLen>.rules[0].finds[1].kind ==
              static_cast<std::uint32_t>(bdb::foreign::bdb_find_term_kind::BDB_FIND_TERM_KIND_MEASURE));
static_assert(std::same_as<decltype(std::declval<bdb::row_of<WindowLen>>().len), std::uint64_t>);

static_assert(NamedDownAt.rules[0].atom_count == 2);
static_assert(NamedDownAt.rules[0].atoms[1].source == bdb::atom_source::edb);
static_assert(NamedDownAt.rules[0].atoms[1].id == 0);
static_assert(NamedDownAt.rules[0].atoms[1].binding_count == 2);
static_assert(NamedDownAt.rules[0].atoms[1].bindings[0].field == 0);
static_assert(NamedDownAt.rules[0].atoms[1].bindings[0].term.var == 0);
static_assert(NamedDownAt.rules[0].atoms[1].bindings[1].field == 1);
static_assert(NamedDownAt.rules[0].atoms[1].bindings[1].term.var == 2);
static_assert(NamedDownAt.rules[0].finds[0].over == 2);
static_assert(NamedDownAt.head[0].answer == bdb::field_class{bdb::value_kind::string, 0});
static_assert(std::same_as<decltype(std::declval<bdb::row_of<NamedDownAt>>().name), std::string>);

namespace {

consteval auto mid_rule() {
	return [](auto r) consteval {
		auto vars = r.vars(Service);
		return r.match(Service, {.id = vars.id}).find({.id = vars.id});
	};
}

}

inline constexpr auto FiveInterior =
    bdb::query(Uptime)
        .interior<"mid">(mid_rule(), mid_rule(), mid_rule(), mid_rule(), mid_rule())
        .rule([](auto r) consteval {
	        auto vars = r.vars(Service);
	        return r.template interior<"mid">(bdb::bind<"id">(vars.id)).find({}, bdb::as<"id">(vars.id));
        });

static_assert(FiveInterior.interiors.size() == 1);
static_assert(FiveInterior.interiors[0].rule_count == 5);

auto main() -> int {
	std::println("pass: recipe-1 query IR lowers to the lower.ts shape "
	             "(vars/atoms/conditions/params/head, all pinned)");
	std::println("pass: row_of/params_of synthesize named, exactly-typed "
	             "products");
	return 0;
}
