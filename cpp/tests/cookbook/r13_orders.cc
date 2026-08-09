// Cookbook recipe 13 — State machines (ts/COOKBOOK.md §13): states are a
// discriminated union; per-state data lives in arms; and the conditional
// reference target — a reference to "an order THAT IS shipped" — is one
// selected statement, the statement SQL cannot write: `mirrors(
// Shipment.order ~ Order.where(state: Shipped).id)`. History accretes: a
// Shipped order keeps its Placement (the one-way containment). Transition
// predicates are host code under the generation witness (recipe 20); the
// schema pins the states, not the paths.
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r13 <64-hex>"); shipped (the handle-literal + join query) prepares
// AND answers the recipe's own semantics; a Shipped order without its
// Shipment is commit-rejected (totality) and a Shipment referencing a
// Cart order is commit-rejected (validity) — the transition and its
// evidence commit together.
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

inline constexpr auto State = bdb::closed<"State", "Cart", "Placed", "Shipped">();

struct OrderRow {
	[[= bdb::fresh]] std::uint64_t id;

	bdb::ref_to<State.id> state;
};

struct PlacementRow {
	std::uint64_t order;
	std::int64_t at;
};

struct ShipmentRow {
	std::uint64_t order;
	std::string carrier;
	std::int64_t at;
};

inline constexpr auto Order = bdb::relation<"Order", OrderRow>;
inline constexpr auto Placement = bdb::relation<"Placement", PlacementRow>;
inline constexpr auto Shipment = bdb::relation<"Shipment", ShipmentRow>;

inline constexpr auto Orders =
    bdb::schema<"Orders">(State, Order, Placement, Shipment,

                          bdb::contained(bdb::on(Order.state), bdb::on(State.id)), bdb::key(Placement.order), bdb::key(Shipment.order),

                          // History accretes: a Shipped order keeps its Placement — one-way
                          // containment admits arms from earlier states surviving the
                          // transition.
                          bdb::contained(bdb::on(Placement.order), bdb::on(Order.id)),

                          // The conditional target, both ways: every Shipment references an
                          // order THAT IS Shipped (validity), and every Shipped order has its
                          // Shipment (totality) — the transition and its evidence commit
                          // together.
                          bdb::mirrors(bdb::on(Shipment.order), bdb::on(bdb::where(Order, {.state = State.Shipped}), Order.id)));

// The handle literal in the match record + the join into the arm.
inline constexpr auto Shipped = bdb::query(Orders).rule([](auto r) consteval {
	auto o = r.vars(Order);
	auto s = r.vars(Shipment);
	return r
	    .match(Order,
	           {
	               .id = o.id,
	               .state = State.Shipped,
	           })
	    .match(Shipment,
	           {
	               .order = o.id,
	               .carrier = s.carrier,
	           })
	    .find({
	        .id = o.id,
	        .carrier = s.carrier,
	    });
});

namespace {

struct CaseResult {
	std::string name;
	bool passed;
};

auto golden_of(std::string_view fixtures, std::string_view recipe) -> std::optional<std::string> {
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

auto slurp(std::string_view path) -> std::optional<std::string> {
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

auto make_store_dir() -> std::optional<std::filesystem::path> {
	auto code = std::error_code{};
	auto const root = std::filesystem::temp_directory_path(code);
	if (code) {
		return std::nullopt;
	}
	auto device = std::random_device{};
	auto const dir = root / std::format("bumbledb-cookbook-r13-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t cart;
	std::uint64_t shipped;
};

/// One order still in the Cart; one Shipped order carrying BOTH arms —
/// its surviving Placement (history accretes) and the Shipment evidence
/// the two-way mirrors demands in the same commit.
auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto ids = SeedIds{};
		auto const mint = [&](std::uint64_t& out) -> std::expected<bool, bdb::Error> {
			return tx.alloc(Order.id).transform([&](std::uint64_t minted) {
				out = minted;
				return true;
			});
		};
		auto rows_land = mint(ids.cart)
		                     .and_then([&](bool) {
			                     return mint(ids.shipped);
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Order, OrderRow{.id = ids.cart, .state = State.Cart});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Order, OrderRow{.id = ids.shipped, .state = State.Shipped});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Placement, PlacementRow{.order = ids.shipped, .at = 5});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Shipment, ShipmentRow{.order = ids.shipped, .carrier = std::string{"ups"}, .at = 9});
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
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r13") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r13 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r13 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Orders);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Orders", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r13 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "the transition and its evidence commit together "
	            "(Shipped + Placement + Shipment in one delta)",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	// TOTALITY: a Shipped order without its Shipment evidence.
	auto unevidenced = db->write([&](bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::monostate>, bdb::Error> {
		auto landed = tx.alloc(Order.id).and_then([&](std::uint64_t order) {
			return tx.insert(Order, OrderRow{.id = order, .state = State.Shipped});
		});
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
	results.push_back(CaseResult{
	    .name = "a Shipped order without its Shipment is commit-rejected "
	            "(totality)",
	    .passed = !unevidenced.has_value() && unevidenced.error().kind() == bdb::ErrorKind::CommitRejected &&
	              !unevidenced.error().violations().empty(),
	});

	// VALIDITY: a Shipment referencing an order that is NOT Shipped.
	auto premature = db->write([&](bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::monostate>, bdb::Error> {
		auto landed = tx.insert(Shipment, ShipmentRow{.order = ids->cart, .carrier = std::string{"dhl"}, .at = 11});
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
	results.push_back(CaseResult{
	    .name = "a Shipment referencing a Cart order is commit-rejected "
	            "(validity)",
	    .passed =
	        !premature.has_value() && premature.error().kind() == bdb::ErrorKind::CommitRejected && !premature.error().violations().empty(),
	});

	auto shipped = db->prepare<Shipped>();
	results.push_back(CaseResult{
	    .name = "shipped prepares through the engine validator",
	    .passed = shipped.has_value(),
	});
	if (!shipped.has_value()) {
		return;
	}

	// The handle literal + join: only the Shipped order answers, with
	// its carrier.
	auto answers = db->execute(*shipped, {});
	results.push_back(CaseResult{
	    .name = "shipped answers {(shipped order, ups)}",
	    .passed = answers.has_value() && answers->size() == 1 && answers->rows().front().id == ids->shipped &&
	              answers->rows().front().carrier == "ups",
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r13_orders <fixtures-file>");
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
