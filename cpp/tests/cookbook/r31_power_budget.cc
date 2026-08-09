// Cookbook recipe 31 — The power budget (ts/COOKBOOK.md §31): the
// weighted capacity law with the pinned-column idiom. The two-column
// containment IS the join stated as a law (a device's watts provably
// equals its model's at every commit), and Σ watts over a pool's devices
// stays within the pool's own supply — `capacity(target, weigh(field),
// within(0, ref(field)), source)`, the operator read order.
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r31 <64-hex>"); utilization is a QUERY, never a column — `draw` folds
// `sum` over a plain scalar variable (the recipe's own spelling), with
// count / max / arg_max beside it (the full head-op roster against the
// real engine); an over-budget commit is rejected citing the capacity
// statement; and the top-up write runs through Db::write_witnessed (the
// snapshot-and-tx-in-one-callback lane).
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

struct PoolRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::uint64_t supply;
};

struct ModelRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::uint64_t watts;
};

struct DeviceRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::uint64_t pool;
	std::uint64_t model;
	std::uint64_t watts;
};

inline constexpr auto Pool = bdb::relation<"Pool", PoolRow>;
inline constexpr auto Model = bdb::relation<"Model", ModelRow>;
inline constexpr auto Device = bdb::relation<"Device", DeviceRow>;

inline constexpr auto Racks = bdb::schema<"Racks">(
    Pool, Model, Device,

    bdb::contained(bdb::on(Device.pool), bdb::on(Pool.id)),

    // The pinned column: a device's watts provably equals its model's —
    // the two-column containment IS the join, stated as a law. The
    // superkey it targets is deliberate write-amplification rent.
    bdb::key(Model.id, Model.watts), bdb::contained(bdb::on(Device.model, Device.watts), bdb::on(Model.id, Model.watts)),

    // Σ watts over a pool's devices stays within the pool's own supply:
    bdb::capacity(bdb::on(Pool.id), bdb::weigh(Device.watts), bdb::within(0, bdb::ref(Pool.supply)), bdb::on(Device.pool)));

// utilization is a query, never a column (the ledger's law, recipe 19):
// the sum folds a PLAIN SCALAR variable — the recipe's own spelling.
inline constexpr auto Draw = bdb::query(Racks).rule([](auto r) consteval {
	auto vars = r.vars(Device);
	return r
	    .match(Device,
	           {
	               .id = vars.id,
	               .pool = vars.pool,
	               .watts = vars.watts,
	           })
	    .find(
	        {
	            .pool = vars.pool,
	        },
	        bdb::sum<"total">(vars.watts));
});

// The fold roster beside it: fleet size (nullary count) and peak draw
// (max over the scalar).
inline constexpr auto FleetFacts = bdb::query(Racks).rule([](auto r) consteval {
	auto vars = r.vars(Device);
	return r
	    .match(Device,
	           {
	               .id = vars.id,
	               .pool = vars.pool,
	               .watts = vars.watts,
	           })
	    .find(
	        {
	            .pool = vars.pool,
	        },
	        bdb::count<"n">(), bdb::max<"peak">(vars.watts));
});

// The Arg restriction rides its own head (the engine's rule: Arg terms
// and fold aggregates may not mix): the peak device per pool.
inline constexpr auto TopDevice = bdb::query(Racks).rule([](auto r) consteval {
	auto vars = r.vars(Device);
	return r
	    .match(Device,
	           {
	               .id = vars.id,
	               .pool = vars.pool,
	               .watts = vars.watts,
	           })
	    .find(
	        {
	            .pool = vars.pool,
	        },
	        bdb::arg_max<"top">(vars.id, vars.watts));
});

namespace {

struct CaseResult {
	std::string name;
	bool passed;
};

/// The golden of one recipe: the fixtures file is one `rNN <64-hex>` line
/// per recipe (ts/test/cookbook.test.ts reads the same file).
[[nodiscard]] auto golden_of(std::string_view fixtures, std::string_view recipe) -> std::optional<std::string> {
	for (auto const line_range : std::views::split(fixtures, '\n')) {
		auto const line = std::string_view{line_range};
		if (!line.starts_with(recipe)) {
			continue;
		}
		auto const rest = line.substr(recipe.size());
		if (!rest.starts_with(' ')) {
			continue;
		}
		auto hex = rest.substr(1);
		while (!hex.empty() && (hex.back() == '\r' || hex.back() == ' ')) {
			hex.remove_suffix(1);
		}
		if (hex.size() != 64) {
			return std::nullopt;
		}
		return std::string{hex};
	}
	return std::nullopt;
}

[[nodiscard]] auto slurp(std::string_view path) -> std::optional<std::string> {
	auto stream = std::ifstream{std::string{path}, std::ios::binary | std::ios::ate};
	if (!stream) {
		return std::nullopt;
	}
	auto const size = stream.tellg();
	if (size < 0) {
		return std::nullopt;
	}
	auto text = std::string(static_cast<std::size_t>(size), '\0');
	stream.seekg(0);
	stream.read(text.data(), size);
	if (!stream) {
		return std::nullopt;
	}
	return text;
}

[[nodiscard]] auto make_store_dir() -> std::optional<std::filesystem::path> {
	auto code = std::error_code{};
	auto const root = std::filesystem::temp_directory_path(code);
	if (code) {
		return std::nullopt;
	}
	auto device = std::random_device{};
	auto const dir = root / std::format("bumbledb-cookbook-r31-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t pool_a;
	std::uint64_t pool_b;
	std::uint64_t model_40;
	std::uint64_t model_25;
	std::uint64_t model_10;
	std::uint64_t device_40;
};

/// Pools A (supply 100) and B (supply 30); models at 40/25/10 W; A runs
/// a 40 W and a 25 W device, B a 10 W one.
[[nodiscard]] auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto ids = SeedIds{};
		auto const mint = [&](auto coordinate, std::uint64_t& out) -> std::expected<bool, bdb::Error> {
			return tx.alloc(coordinate).transform([&](std::uint64_t minted) {
				out = minted;
				return true;
			});
		};
		auto device_25 = std::uint64_t{0};
		auto device_10 = std::uint64_t{0};
		auto rows_land =
		    mint(Pool.id, ids.pool_a)
		        .and_then([&](bool) {
			        return mint(Pool.id, ids.pool_b);
		        })
		        .and_then([&](bool) {
			        return mint(Model.id, ids.model_40);
		        })
		        .and_then([&](bool) {
			        return mint(Model.id, ids.model_25);
		        })
		        .and_then([&](bool) {
			        return mint(Model.id, ids.model_10);
		        })
		        .and_then([&](bool) {
			        return mint(Device.id, ids.device_40);
		        })
		        .and_then([&](bool) {
			        return mint(Device.id, device_25);
		        })
		        .and_then([&](bool) {
			        return mint(Device.id, device_10);
		        })
		        .and_then([&](bool) {
			        return tx.insert(Pool, PoolRow{.id = ids.pool_a, .supply = 100});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Pool, PoolRow{.id = ids.pool_b, .supply = 30});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Model, ModelRow{.id = ids.model_40, .watts = 40});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Model, ModelRow{.id = ids.model_25, .watts = 25});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Model, ModelRow{.id = ids.model_10, .watts = 10});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Device, DeviceRow{.id = ids.device_40, .pool = ids.pool_a, .model = ids.model_40, .watts = 40});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Device, DeviceRow{.id = device_25, .pool = ids.pool_a, .model = ids.model_25, .watts = 25});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Device, DeviceRow{.id = device_10, .pool = ids.pool_b, .model = ids.model_10, .watts = 10});
		        });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(ids);
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<SeedIds>>(*written).value;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r31") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r31 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r31 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Racks);
	if (!db.has_value()) {
		results.push_back(CaseResult{.name = "Db::ephemeral admits Racks", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r31 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "pools, models, and the in-budget fleet commit",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	auto draw = db->prepare<Draw>();
	auto fleet = db->prepare<FleetFacts>();
	auto top = db->prepare<TopDevice>();
	results.push_back(CaseResult{
	    .name = "draw / fleetFacts / topDevice prepare through the engine "
	            "validator",
	    .passed = draw.has_value() && fleet.has_value() && top.has_value(),
	});
	if (!draw.has_value() || !fleet.has_value() || !top.has_value()) {
		return;
	}

	// The scalar-sum utilization, one Db::execute call, host-sorted by
	// descending total (the keys-as-data comparator).
	using DrawRow = bdb::row_of<Draw>;
	auto totals = db->execute(*draw, {}).transform([](bdb::Answers<Draw> answers) {
		auto rows = std::vector<DrawRow>{};
		for (auto const& row : answers.rows()) {
			rows.push_back(row);
		}
		std::ranges::sort(rows, bdb::by(bdb::desc(&DrawRow::total)));
		return rows;
	});
	results.push_back(CaseResult{
	    .name = "draw answers {(A, 65), (B, 10)}, descending by total",
	    .passed = totals.has_value() && totals->size() == 2 && (*totals)[0].pool == ids->pool_a && (*totals)[0].total == 65 &&
	              (*totals)[1].pool == ids->pool_b && (*totals)[1].total == 10,
	});

	// The fold roster: count / max per pool.
	using FleetRow = bdb::row_of<FleetFacts>;
	auto facts = db->execute(*fleet, {}).transform([](bdb::Answers<FleetFacts> answers) {
		auto rows = std::vector<FleetRow>{};
		for (auto const& row : answers.rows()) {
			rows.push_back(row);
		}
		std::ranges::sort(rows, bdb::by(&FleetRow::pool));
		return rows;
	});
	auto facts_pass = facts.has_value() && facts->size() == 2;
	if (facts_pass) {
		auto const& a = (*facts)[0].pool == ids->pool_a ? (*facts)[0] : (*facts)[1];
		auto const& b = (*facts)[0].pool == ids->pool_b ? (*facts)[0] : (*facts)[1];
		facts_pass = a.n == 2 && a.peak == 40 && b.n == 1 && b.peak == 10;
	}
	results.push_back(CaseResult{
	    .name = "fleetFacts answers count/max per pool",
	    .passed = facts_pass,
	});

	// The Arg restriction: pool A's peak device is the 40 W one.
	auto top_a = db->execute(*top, {}).transform([&](bdb::Answers<TopDevice> answers) {
		auto device = std::optional<std::uint64_t>{};
		for (auto const& row : answers.rows()) {
			if (row.pool == ids->pool_a) {
				device = row.top;
			}
		}
		return device;
	});
	results.push_back(CaseResult{
	    .name = "topDevice answers pool A's 40 W device (argMax)",
	    .passed = top_a.has_value() && top_a->has_value() && **top_a == ids->device_40,
	});

	// The over-budget commit refuses citing the capacity statement:
	// B would carry 10 + 40 = 50 > 30.
	auto overdraw = db->write([&](bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::monostate>, bdb::Error> {
		auto landed = tx.alloc(Device.id).and_then([&](std::uint64_t minted) {
			return tx.insert(Device, DeviceRow{.id = minted, .pool = ids->pool_b, .model = ids->model_40, .watts = 40});
		});
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
	auto overdraw_cited = !overdraw.has_value() && overdraw.error().kind() == bdb::ErrorKind::CommitRejected;
	if (overdraw_cited) {
		auto const violations = overdraw.error().violations();
		overdraw_cited = !violations.empty() && std::ranges::any_of(violations, [](bdb::Violation const& violation) {
			return violation.kind == bdb::StatementKind::Capacity;
		});
	}
	results.push_back(CaseResult{
	    .name = "the over-budget device is commit-rejected citing the "
	            "capacity law",
	    .passed = overdraw_cited,
	});

	// The witnessed top-up (Db::write_witnessed): read pool B's current
	// draw off the WITNESSING snapshot, fill exactly to the ceiling, and
	// commit — snapshot and tx in one callback, retry owned by the loop.
	auto witnessed = db->write_witnessed(
	    [&](bdb::Snapshot& snap, bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::uint64_t, std::monostate>, bdb::Error> {
		    auto used = snap.execute(*draw, {}).transform([&](bdb::Answers<Draw> answers) {
			    auto total = std::uint64_t{0};
			    for (auto const& row : answers.rows()) {
				    if (row.pool == ids->pool_b) {
					    total = row.total;
				    }
			    }
			    return total;
		    });
		    if (!used.has_value()) {
			    return std::unexpected{std::move(used).error()};
		    }
		    auto const headroom = std::uint64_t{30} - *used;
		    if (headroom < 10) {
			    return bdb::abandon();
		    }
		    // The 10 W model fills part of the headroom.
		    auto landed = tx.alloc(Device.id).and_then([&](std::uint64_t minted) {
			    return tx.insert(Device, DeviceRow{.id = minted, .pool = ids->pool_b, .model = ids->model_10, .watts = 10});
		    });
		    if (!landed.has_value()) {
			    return std::unexpected{std::move(landed).error()};
		    }
		    return bdb::commit(*used + 10);
	    });
	auto witnessed_pass = witnessed.has_value() && std::holds_alternative<bdb::Committed<std::uint64_t>>(*witnessed) &&
	                      std::get<bdb::Committed<std::uint64_t>>(*witnessed).value == 20;
	results.push_back(CaseResult{
	    .name = "write_witnessed tops pool B up to 20 W (snapshot + tx in "
	            "one callback)",
	    .passed = witnessed_pass,
	});

	// The topped-up utilization reads back.
	auto after = db->execute(*draw, {}).transform([&](bdb::Answers<Draw> answers) {
		auto total = std::uint64_t{0};
		for (auto const& row : answers.rows()) {
			if (row.pool == ids->pool_b) {
				total = row.total;
			}
		}
		return total;
	});
	results.push_back(CaseResult{
	    .name = "pool B draws 20 W after the witnessed top-up",
	    .passed = after.has_value() && *after == 20,
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r31_power_budget <fixtures-file>");
		return 1;
	}

	auto results = std::vector<CaseResult>{};
	run_cases(std::string_view{arguments[1]}, results);

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
