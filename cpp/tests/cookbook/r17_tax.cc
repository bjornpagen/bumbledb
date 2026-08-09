// Cookbook recipe 17 — Federal income tax (ts/COOKBOOK.md §17): brackets
// are intervals over money; the top bracket is a RAY (end == MAX denotes
// [s, ∞), an honest value of the representation, not a sentinel — the
// point-domain law); regimes key on (year, status) — a key INCLUDING a
// closed-handle column; and proration happens at write time, never at
// query time (the representation move that deletes clip-at-query,
// gravestone recipe 23).
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r17 <64-hex>"); marginal — the two-atom join binding a param AT the
// closed-handle field ({status: r.param("s")}) — prepares AND answers the
// recipe's own semantics including the ray bracket; a second regime for
// one (year, status) and an Earned fact outside residency are both
// commit-rejected.
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

inline constexpr auto Status = bdb::closed<"Status", "Single", "MarriedJoint", "HeadOfHousehold">();

struct RegimeRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::int64_t year;
	bdb::ref_to<Status.id> status;
};

struct BracketRow {
	std::uint64_t regime;
	bdb::interval<std::int64_t> income;
	std::int64_t rate_bps;
};

struct ResidencyRow {
	std::uint64_t person;
	bdb::interval<std::int64_t> span;
};

// Split at write: an Earned fact never spans a year boundary — writers
// split (prorate) at the boundary, so no reader ever clips.
struct EarnedRow {
	std::uint64_t person;
	std::uint64_t regime;
	bdb::interval<std::int64_t> span;
	std::int64_t minor;
};

inline constexpr auto Regime = bdb::relation<"Regime", RegimeRow>;
inline constexpr auto Bracket = bdb::relation<"Bracket", BracketRow>;
inline constexpr auto Residency = bdb::relation<"Residency", ResidencyRow>;
inline constexpr auto Earned = bdb::relation<"Earned", EarnedRow>;

inline constexpr auto Tax =
    bdb::schema<"Tax">(Status, Regime, Bracket, Residency, Earned,

                       bdb::contained(bdb::on(Regime.status), bdb::on(Status.id)),

                       // One regime per (year, filing status) — the key includes the
                       // closed-handle column.
                       bdb::key(Regime.year, Regime.status),

                       bdb::contained(bdb::on(Bracket.regime), bdb::on(Regime.id)),

                       // Brackets are disjoint per regime. Seed data conventionally covers
                       // [0, ∞) and the top bracket is a ray, but this key proves
                       // disjointness only.
                       bdb::key(Bracket.regime, Bracket.income),

                       bdb::contained(bdb::on(Earned.regime), bdb::on(Regime.id)),

                       bdb::key(Residency.person, Residency.span),

                       // Residency exclusion: income counts only where earned inside a
                       // residency period — pointwise coverage, the same judgment as recipe
                       // 15's. This pair statement is also what puts the two bare `person`
                       // columns in one (generator-less) class: "Residency.person", by least
                       // coordinate.
                       bdb::contained(bdb::on(Earned.person, Earned.span), bdb::on(Residency.person, Residency.span)));

// the marginal bracket (membership probes the disjoint bracket set). Tax
// owed is host arithmetic over the bracket walk — arithmetic beyond the
// measure is refused (the ledger). The `status` slot binds a PARAM at the
// closed-handle field.
inline constexpr auto Marginal = bdb::query(Tax).rule([](auto r) consteval {
	auto regime = r.vars(Regime);
	auto bracket = r.vars(Bracket);
	return r
	    .match(Regime,
	           {
	               .id = regime.id,
	               .year = bdb::param<"y">(),
	               .status = bdb::param<"s">(),
	           })
	    .match(Bracket,
	           {
	               .regime = regime.id,
	               .income = bracket.income,
	               .rate_bps = bracket.rate_bps,
	           })
	    .where(bdb::point_in(bdb::param<"taxable">(), bracket.income))
	    .find({
	        .rate_bps = bracket.rate_bps,
	    });
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
	auto const dir = root / std::format("bumbledb-cookbook-r17-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

/// The ray convention: [s, ∞) is spelled [s, MAX) — an honest value of
/// the representation, not a sentinel.
inline constexpr auto ray_top = std::numeric_limits<std::int64_t>::max();

/// One 2024/Single regime with three disjoint brackets (the top one a
/// ray), one residency [0,100) for person 7, and one Earned fact inside
/// it.
///
///   [0,50) @ 1000bps    [50,100) @ 2000bps    [100, ∞) @ 3000bps
[[nodiscard]] auto seed(bdb::Db& db) -> std::optional<std::uint64_t> {
	using Decision = bdb::WriteDecision<std::uint64_t, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto regime = tx.alloc(Regime.id);
		if (!regime.has_value()) {
			return std::unexpected{std::move(regime).error()};
		}
		auto rows_land =
		    tx.insert(Regime, RegimeRow{.id = *regime, .year = 2024, .status = Status.Single})
		        .and_then([&](bool) {
			        return tx.insert(
			            Bracket, BracketRow{.regime = *regime, .income = bdb::interval<std::int64_t>::literal(0, 50), .rate_bps = 1000});
		        })
		        .and_then([&](bool) {
			        return tx.insert(
			            Bracket, BracketRow{.regime = *regime, .income = bdb::interval<std::int64_t>::literal(50, 100), .rate_bps = 2000});
		        })
		        .and_then([&](bool) {
			        return tx.insert(
			            Bracket,
			            BracketRow{.regime = *regime, .income = bdb::interval<std::int64_t>::literal(100, ray_top), .rate_bps = 3000});
		        })
		        .and_then([&](bool) {
			        return tx.insert(Residency, ResidencyRow{.person = 7, .span = bdb::interval<std::int64_t>::literal(0, 100)});
		        })
		        .and_then([&](bool) {
			        return tx.insert(
			            Earned,
			            EarnedRow{.person = 7, .regime = *regime, .span = bdb::interval<std::int64_t>::literal(10, 20), .minor = 5000});
		        });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(*regime);
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<std::uint64_t>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<std::uint64_t>>(*written).value;
}

/// marginal(y, s, taxable), rates sorted (answers are sets; the host
/// sorts). The status param crosses as the handle's declaration-order
/// row id — the closed-handle bind domain is the u64 handle column.
[[nodiscard]] auto marginal_rates(bdb::Db& db, bdb::Prepared<Marginal>& prepared, std::int64_t year, std::uint64_t status, std::int64_t taxable)
    -> std::optional<std::vector<std::int64_t>> {
	auto result = db.execute(prepared, {.y = year, .s = status, .taxable = taxable}).transform([](bdb::Answers<Marginal> answers) {
		auto rates = std::vector<std::int64_t>{};
		for (auto const& row : answers.rows()) {
			rates.push_back(row.rate_bps);
		}
		std::ranges::sort(rates);
		return rates;
	});
	if (!result.has_value()) {
		return std::nullopt;
	}
	return *std::move(result);
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r17") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r17 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r17 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Tax);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Tax", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r17 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto const regime = seed(*db);
	results.push_back(CaseResult{
	    .name = "the regime, its brackets (top one a ray), the residency, "
	            "and one covered Earned fact commit",
	    .passed = regime.has_value(),
	});
	if (!regime.has_value()) {
		return;
	}

	auto marginal = db->prepare<Marginal>();
	results.push_back(CaseResult{
	    .name = "marginal prepares through the engine validator (a param "
	            "bound at the closed-handle field)",
	    .passed = marginal.has_value(),
	});
	if (!marginal.has_value()) {
		return;
	}

	// The membership probe against the disjoint bracket set.
	auto const at_30 = marginal_rates(*db, *marginal, 2024, Status.Single.index, 30);
	results.push_back(CaseResult{
	    .name = "marginal(2024, Single, 30) answers {1000}",
	    .passed = at_30.has_value() && *at_30 == std::vector<std::int64_t>{1000},
	});
	auto const at_75 = marginal_rates(*db, *marginal, 2024, Status.Single.index, 75);
	results.push_back(CaseResult{
	    .name = "marginal(2024, Single, 75) answers {2000}",
	    .passed = at_75.has_value() && *at_75 == std::vector<std::int64_t>{2000},
	});
	auto const at_top = marginal_rates(*db, *marginal, 2024, Status.Single.index, 1'000'000);
	results.push_back(CaseResult{
	    .name = "marginal(2024, Single, 1000000) answers {3000} (the ray "
	            "holds every point above its start)",
	    .passed = at_top.has_value() && *at_top == std::vector<std::int64_t>{3000},
	});
	auto const wrong_status = marginal_rates(*db, *marginal, 2024, Status.MarriedJoint.index, 30);
	results.push_back(CaseResult{
	    .name = "marginal(2024, MarriedJoint, 30) answers the empty set "
	            "(no regime for that filing status)",
	    .passed = wrong_status.has_value() && wrong_status->empty(),
	});

	// A second regime for one (year, filing status) violates the key that
	// includes the closed-handle column.
	auto duplicated = db->write([&](bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::monostate>, bdb::Error> {
		auto landed = tx.alloc(Regime.id).and_then([&](std::uint64_t minted) {
			return tx.insert(Regime, RegimeRow{.id = minted, .year = 2024, .status = Status.Single});
		});
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
	results.push_back(CaseResult{
	    .name = "a second 2024/Single regime is commit-rejected (one "
	            "regime per (year, status))",
	    .passed = !duplicated.has_value() && duplicated.error().kind() == bdb::ErrorKind::CommitRejected &&
	              !duplicated.error().violations().empty(),
	});

	// Income earned outside every residency period violates the
	// residency-exclusion coverage.
	auto outside = db->write([&](bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::monostate>, bdb::Error> {
		auto landed = tx.insert(
		    Earned, EarnedRow{.person = 7, .regime = *regime, .span = bdb::interval<std::int64_t>::literal(150, 160), .minor = 900});
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
	results.push_back(CaseResult{
	    .name = "an Earned fact outside residency is commit-rejected "
	            "(income counts only where earned inside a residency "
	            "period)",
	    .passed = !outside.has_value() && outside.error().kind() == bdb::ErrorKind::CommitRejected && !outside.error().violations().empty(),
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r17_tax <fixtures-file>");
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
