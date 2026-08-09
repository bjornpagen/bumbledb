// §34 / TODO_CPP §21 — a wrong parameter name at execute is a compile
// error: the params product is synthesized from the query's registry
// (one member per param, named per param), so a designated initializer
// naming anything else cannot compile. DownAt registers exactly "t".
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

                                                     bdb::contained(bdb::on(Outage.service), bdb::on(Service.id)));

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

// What `snap.execute(prepared, {...})` takes — with the wrong name.
[[nodiscard]] consteval auto misuse() -> bool {
	auto const params = bdb::params_of<DownAt>{.at = std::int64_t{42}};
	return sizeof params != 0;
}

static_assert(misuse());
