import std;
import bumbledb;

inline constexpr auto Kind = bdb::closed<"Kind", "Unit", "Pair">();

struct LedgerRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct ZoneRow {
	std::uint64_t ledger;
	bdb::ref_to<Kind.id> kind;
	bdb::interval<std::uint64_t> at;
};

struct UnitSlotRow {
	std::uint64_t ledger;
	bdb::interval<std::uint64_t, 1> at;
	std::uint64_t entry;
};

struct PairSlotRow {
	std::uint64_t ledger;
	bdb::interval<std::uint64_t, 2> at;
	std::uint64_t entry;
};

inline constexpr auto Ledger = bdb::relation<"Ledger", LedgerRow>;
inline constexpr auto Zone = bdb::relation<"Zone", ZoneRow>;
inline constexpr auto UnitSlot = bdb::relation<"UnitSlot", UnitSlotRow>;
inline constexpr auto PairSlot = bdb::relation<"PairSlot", PairSlotRow>;

inline constexpr auto ZoneLedger = bdb::schema<"ZoneLedger">(
    Kind, Ledger, Zone, UnitSlot, PairSlot,

    bdb::contained(bdb::on(Zone.ledger), bdb::on(Ledger.id)), bdb::contained(bdb::on(Zone.kind), bdb::on(Kind.id)),

    bdb::key(Zone.ledger, Zone.at), bdb::key(UnitSlot.ledger, UnitSlot.at), bdb::key(PairSlot.ledger, PairSlot.at),

    bdb::mirrors(bdb::on(bdb::where(Zone, {.kind = Kind.Unit}), Zone.ledger, Zone.at), bdb::on(UnitSlot.ledger, UnitSlot.at)),
    bdb::mirrors(bdb::on(bdb::where(Zone, {.kind = Kind.Pair}), Zone.ledger, Zone.at), bdb::on(PairSlot.ledger, PairSlot.at)));

static_assert(bdb::interval<std::uint64_t, 1>::make(4, 5).has_value());
static_assert(!bdb::interval<std::uint64_t, 1>::make(4, 6).has_value());
static_assert(bdb::interval<std::uint64_t, 1>::make(4, 6).error() == bdb::TypeError::IntervalWidth);
static_assert(bdb::interval<std::uint64_t, 2>::make(4, 6).has_value());

namespace {

struct CaseResult {
	std::string name;
	bool passed;
};

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
	auto const dir = root / std::format("bumbledb-cookbook-r29-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

[[nodiscard]] auto seed(bdb::Db& db) -> std::optional<std::uint64_t> {
	using Decision = bdb::WriteDecision<std::uint64_t, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto ledger = tx.alloc(Ledger.id);
		if (!ledger.has_value()) {
			return std::unexpected{std::move(ledger).error()};
		}
		auto rows_land =
		    tx.insert(Ledger, LedgerRow{.id = *ledger, .name = std::string{"main"}})
		        .and_then([&](bool) {
			        return tx.insert(Zone,
			                         ZoneRow{.ledger = *ledger, .kind = Kind.Unit, .at = bdb::interval<std::uint64_t>::literal(4, 6)});
		        })
		        .and_then([&](bool) {
			        return tx.insert(UnitSlot,
			                         UnitSlotRow{.ledger = *ledger, .at = bdb::interval<std::uint64_t, 1>::literal(4, 5), .entry = 7});
		        })
		        .and_then([&](bool) {
			        return tx.insert(UnitSlot,
			                         UnitSlotRow{.ledger = *ledger, .at = bdb::interval<std::uint64_t, 1>::literal(5, 6), .entry = 8});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Zone,
			                         ZoneRow{.ledger = *ledger, .kind = Kind.Pair, .at = bdb::interval<std::uint64_t>::literal(10, 12)});
		        })
		        .and_then([&](bool) {
			        return tx.insert(PairSlot,
			                         PairSlotRow{.ledger = *ledger, .at = bdb::interval<std::uint64_t, 2>::literal(10, 12), .entry = 9});
		        });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(*ledger);
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<std::uint64_t>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<std::uint64_t>>(*written).value;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r29") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r29 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r29 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), ZoneLedger);
	if (!db.has_value()) {
		results.push_back(CaseResult{.name = "Db::ephemeral admits ZoneLedger", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r29 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto const ledger = seed(*db);
	results.push_back(CaseResult{
	    .name = "one Unit zone [4,6) beside slots [4,5)+[5,6) commits "
	            "(coalescing insensitivity)",
	    .passed = ledger.has_value(),
	});
	if (!ledger.has_value()) {
		return;
	}

	auto uncovered = db->write([&](bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::monostate>, bdb::Error> {
		auto landed =
		    tx.insert(UnitSlot, UnitSlotRow{.ledger = *ledger, .at = bdb::interval<std::uint64_t, 1>::literal(20, 21), .entry = 11});
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
	results.push_back(CaseResult{
	    .name = "a unit slot outside every Unit zone is commit-rejected",
	    .passed =
	        !uncovered.has_value() && uncovered.error().kind() == bdb::ErrorKind::CommitRejected && !uncovered.error().violations().empty(),
	});

	auto scanned = db->scan(UnitSlot);
	auto slots = std::vector<bdb::interval<std::uint64_t>>{};
	if (scanned.has_value()) {
		for (auto row = std::size_t{0}; row != scanned->len(); ++row) {
			auto const cell = scanned->cell({.row = row, .column = 1});
			if (cell.has_value() && std::holds_alternative<bdb::interval<std::uint64_t>>(*cell)) {
				slots.push_back(std::get<bdb::interval<std::uint64_t>>(*cell));
			}
		}
	}
	std::ranges::sort(slots, [](auto const& left, auto const& right) { return left.lo() < right.lo(); });
	results.push_back(CaseResult{
	    .name = "Db::scan(UnitSlot) answers [4,5) and [5,6)",
	    .passed = scanned.has_value() && slots.size() == 2 && slots[0] == bdb::interval<std::uint64_t>::literal(4, 5) &&
	              slots[1] == bdb::interval<std::uint64_t>::literal(5, 6),
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r29_zone_ledger <fixtures-file>");
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
