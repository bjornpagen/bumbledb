// Cookbook recipe 1 — the three queries, run against the real engine
// (TODO_CPP §11–§12, §20–§23; ts/COOKBOOK.md §1). The Uptime theory is
// admitted through the schema lane, the cookbook's example data lands
// through tx.alloc + tx.insert, and downAt / overlapping / downtime all
// prepare through the engine's IR validator and return exactly the
// answers the recipe's own semantics dictate:
//
//   services  search, api (fresh ids minted by the engine)
//   outages   search [0,100), search [150,200), api [50,120)
//
//   downAt(t=60)   → {search, api}     (0≤60<100 and 50≤60<120)
//   downAt(t=130)  → {}                (no window holds 130)
//   downAt(t=160)  → {search}          (150≤160<200)
//   overlapping([110,140)) → {(api, [50,120))}   (only [50,120) shares
//                             a point with [110,140); [0,100) and
//                             [150,200) are disjoint from it)
//   downtime       → {(search, 150), (api, 70)}  (100+50 and 70)
//
// Plus the §23 reuse lane (execute_into refills one carrier) and a
// joined query answering a STRING column (the §22 borrowed-view lane —
// exercised under ASan by the asan-ubsan preset).
import std;
import bumbledb;

namespace {

struct CaseResult {
	std::string name;
	bool passed;
};

// Recipe 1's theory (ts/COOKBOOK.md:110-126), through the real elaborator.
struct ServiceRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct OutageRow {
	std::uint64_t service;
	bdb::interval<std::int64_t> window;
};

} // namespace

inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;
inline constexpr auto Outage = bdb::relation<"Outage", OutageRow>;

inline constexpr auto Uptime = bdb::schema<"Uptime">(Service, Outage,

                                                     bdb::contained(bdb::on(Outage.service), bdb::on(Service.id)),

                                                     bdb::key(Outage.service, Outage.window));

// down at instant t (COOKBOOK.md:130-136).
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

// overlapping an incident window (COOKBOOK.md:138-144).
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

// total downtime per service (COOKBOOK.md:146-149).
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

// The join twist: down services by NAME — a string answer column, so the
// §22 borrowed-view contract is exercised end to end.
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

// Outages at least 100 long — the scalar order comparison over the
// measure, proven against the engine (only [0,100) measures 100).
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

[[nodiscard]] auto make_store_dir() -> std::optional<std::filesystem::path> {
	auto code = std::error_code{};
	auto const root = std::filesystem::temp_directory_path(code);
	if (code) {
		return std::nullopt;
	}
	auto device = std::random_device{};
	auto const dir = root / std::format("bumbledb-cookbook-r01q-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t search;
	std::uint64_t api;
};

/// The recipe's example data: two services, three outages.
[[nodiscard]] auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto search = tx.alloc(Service.id);
		if (!search.has_value()) {
			return std::unexpected{std::move(search).error()};
		}
		auto api = tx.alloc(Service.id);
		if (!api.has_value()) {
			return std::unexpected{std::move(api).error()};
		}
		auto rows_land =
		    tx.insert(Service, ServiceRow{.id = *search, .name = std::string{"search"}})
		        .and_then([&](bool) {
			        return tx.insert(Service, ServiceRow{.id = *api, .name = std::string{"api"}});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Outage, OutageRow{.service = *search, .window = bdb::interval<std::int64_t>::literal(0, 100)});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Outage, OutageRow{.service = *search, .window = bdb::interval<std::int64_t>::literal(150, 200)});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Outage, OutageRow{.service = *api, .window = bdb::interval<std::int64_t>::literal(50, 120)});
		        });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(SeedIds{.search = *search, .api = *api});
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<SeedIds>>(*written).value;
}

/// downAt(t), answers sorted (answers are sets; the host sorts).
[[nodiscard]] auto down_services(bdb::Db& db, bdb::Prepared<DownAt>& prepared, std::int64_t at) -> std::optional<std::vector<std::uint64_t>> {
	auto result = db.read([&](bdb::Snapshot& snap) -> std::expected<std::vector<std::uint64_t>, bdb::Error> {
		return snap.execute(prepared, {.t = at}).transform([](bdb::Answers<DownAt> answers) {
			auto services = std::vector<std::uint64_t>{};
			for (auto const& row : answers.rows()) {
				services.push_back(row.service);
			}
			std::ranges::sort(services);
			return services;
		});
	});
	if (!result.has_value()) {
		return std::nullopt;
	}
	return *std::move(result);
}

auto run_cases(std::vector<CaseResult>& results) -> void {
	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r01 queries store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Uptime);
	if (!db.has_value()) {
		results.push_back(CaseResult{.name = "Db::ephemeral admits Uptime", .passed = false});
		return;
	}

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "the recipe's example data commits (alloc + insert)",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	// All three recipe queries prepare through the REAL engine validator.
	auto down_at = db->prepare<DownAt>();
	auto overlapping = db->prepare<Overlapping>();
	auto downtime = db->prepare<Downtime>();
	results.push_back(CaseResult{
	    .name = "downAt / overlapping / downtime all prepare through the "
	            "engine validator",
	    .passed = down_at.has_value() && overlapping.has_value() && downtime.has_value(),
	});
	if (!down_at.has_value() || !overlapping.has_value() || !downtime.has_value()) {
		return;
	}

	// downAt: point membership at three instants.
	auto expected_both = std::vector{ids->search, ids->api};
	std::ranges::sort(expected_both);
	auto const at_60 = down_services(*db, *down_at, 60);
	results.push_back(CaseResult{
	    .name = "downAt(60) answers {search, api}",
	    .passed = at_60.has_value() && *at_60 == expected_both,
	});
	auto const at_130 = down_services(*db, *down_at, 130);
	results.push_back(CaseResult{
	    .name = "downAt(130) answers the empty set",
	    .passed = at_130.has_value() && at_130->empty(),
	});
	auto const at_160 = down_services(*db, *down_at, 160);
	results.push_back(CaseResult{
	    .name = "downAt(160) answers {search}",
	    .passed = at_160.has_value() && *at_160 == std::vector{ids->search},
	});

	// overlapping: one Allen mask, an interval param, an interval answer
	// column.
	auto const incident = bdb::interval<std::int64_t>::literal(110, 140);
	auto overlap_rows =
	    db->read([&](bdb::Snapshot& snap) -> std::expected<std::vector<std::pair<std::uint64_t, bdb::interval<std::int64_t>>>, bdb::Error> {
		    return snap.execute(*overlapping, {.incident = incident}).transform([](bdb::Answers<Overlapping> answers) {
			    auto rows = std::vector<std::pair<std::uint64_t, bdb::interval<std::int64_t>>>{};
			    for (auto const& row : answers.rows()) {
				    rows.emplace_back(row.service, row.window);
			    }
			    std::ranges::sort(rows, {}, [](auto const& row) {
				    return row.first;
			    });
			    return rows;
		    });
	    });
	results.push_back(CaseResult{
	    .name = "overlapping([110,140)) answers {(api, [50,120))}",
	    .passed = overlap_rows.has_value() && overlap_rows->size() == 1 && (*overlap_rows)[0].first == ids->api &&
	              (*overlap_rows)[0].second == bdb::interval<std::int64_t>::literal(50, 120),
	});

	// downtime: sum(duration(window)) per service.
	auto downtime_rows =
	    db->read([&](bdb::Snapshot& snap) -> std::expected<std::vector<std::pair<std::uint64_t, std::uint64_t>>, bdb::Error> {
		    return snap.execute(*downtime, {}).transform([](bdb::Answers<Downtime> answers) {
			    auto rows = std::vector<std::pair<std::uint64_t, std::uint64_t>>{};
			    for (auto const& row : answers.rows()) {
				    rows.emplace_back(row.service, row.downtime);
			    }
			    std::ranges::sort(rows, {}, [](auto const& row) {
				    return row.first;
			    });
			    return rows;
		    });
	    });
	auto expected_downtime = std::vector<std::pair<std::uint64_t, std::uint64_t>>{{ids->search, 150}, {ids->api, 70}};
	std::ranges::sort(expected_downtime, {}, [](auto const& row) {
		return row.first;
	});
	results.push_back(CaseResult{
	    .name = "downtime answers {(search, 150), (api, 70)}",
	    .passed = downtime_rows.has_value() && *downtime_rows == expected_downtime,
	});

	// §23: execute_into refills ONE caller-owned carrier — capacity
	// reused, previous answers replaced.
	auto reused = bdb::Answers<DownAt>{};
	auto const reuse_sizes = db->read([&](bdb::Snapshot& snap) -> std::expected<std::pair<std::size_t, std::size_t>, bdb::Error> {
		return snap.execute_into(*down_at, {.t = std::int64_t{60}}, reused)
		    .transform([&] {
			    return reused.size();
		    })
		    .and_then([&](std::size_t first) -> std::expected<std::pair<std::size_t, std::size_t>, bdb::Error> {
			    return snap.execute_into(*down_at, {.t = std::int64_t{130}}, reused).transform([&] {
				    return std::pair{first, reused.size()};
			    });
		    });
	});
	results.push_back(CaseResult{
	    .name = "execute_into reuses one carrier (2 answers, then 0)",
	    .passed = reuse_sizes.has_value() && reuse_sizes->first == 2 && reuse_sizes->second == 0,
	});

	// The measure order comparison against the engine: only [0,100)
	// measures ≥ 100.
	auto long_outages = db->prepare<LongOutages>();
	auto long_services = long_outages.has_value()
	                         ? db->read([&](bdb::Snapshot& snap) -> std::expected<std::vector<std::uint64_t>, bdb::Error> {
		                           return snap.execute(*long_outages, {}).transform([](bdb::Answers<LongOutages> answers) {
			                           auto services = std::vector<std::uint64_t>{};
			                           for (auto const& row : answers.rows()) {
				                           services.push_back(row.service);
			                           }
			                           std::ranges::sort(services);
			                           return services;
		                           });
	                           })
	                         : std::expected<std::vector<std::uint64_t>, bdb::Error>{std::unexpected{std::move(long_outages).error()}};
	results.push_back(CaseResult{
	    .name = "longOutages (duration >= 100) answers {search}",
	    .passed = long_services.has_value() && *long_services == std::vector{ids->search},
	});

	// The joined STRING answer column (§22 borrowed views, ASan-audited):
	// names are copied out of the borrowed views inside the read.
	auto named = db->prepare<NamedDownAt>();
	auto down_names = named.has_value() ? db->read([&](bdb::Snapshot& snap) -> std::expected<std::vector<std::string>, bdb::Error> {
		return snap.execute(*named, {.t = std::int64_t{60}}).transform([](bdb::Answers<NamedDownAt> answers) {
			auto names = std::vector<std::string>{};
			for (auto const& row : answers.rows()) {
				names.emplace_back(row.name);
			}
			std::ranges::sort(names);
			return names;
		});
	})
	                                    : std::expected<std::vector<std::string>, bdb::Error>{std::unexpected{std::move(named).error()}};
	results.push_back(CaseResult{
	    .name = "namedDownAt(60) answers {api, search} through borrowed "
	            "string views",
	    .passed = down_names.has_value() && *down_names == std::vector<std::string>{"api", "search"},
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main() -> int {
	auto results = std::vector<CaseResult>{};
	run_cases(results);

	auto failures = std::size_t{0};
	for (auto const& result : results) {
		if (result.passed) {
			std::println("pass: {}", result.name);
		} else {
			std::println("FAIL: {}", result.name);
			++failures;
		}
	}
	return failures == 0 ? 0 : 1;
}
