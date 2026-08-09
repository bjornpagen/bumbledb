// Cookbook recipe 4 — Money (TODO_CPP §8, §33; ts/COOKBOOK.md §4): minor
// units as a bare i64 (never floats), the currency as a closed vocabulary
// (`bdb::ref_to<Currency.id>` — physically the u64 handle row id), and the
// balance as a QUERY, never a column: Σ minor grouped by (account,
// currency), the fresh posting id bound in the match so every posting
// weighs in exactly once.
//
// Fingerprint vs the shared golden (fixtures/cookbook-fingerprints.txt,
// line "r04 <64-hex>"), then the totals query prepares through the REAL
// engine validator and answers the recipe's own semantics.
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

inline constexpr auto Currency = bdb::closed<"Currency", "Usd", "Eur", "Gbp">();

struct AccountRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct PostingRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::uint64_t account;
	bdb::ref_to<Currency.id> currency;
	std::int64_t minor;
};

inline constexpr auto Account = bdb::relation<"Account", AccountRow>;
inline constexpr auto Posting = bdb::relation<"Posting", PostingRow>;

inline constexpr auto Money = bdb::schema<"Money">(Currency, Account, Posting,

                                                   bdb::contained(bdb::on(Posting.account), bdb::on(Account.id)),

                                                   // The closed reference resolves (and `Posting.currency` lands in the
                                                   // "Currency.id" generator class).
                                                   bdb::contained(bdb::on(Posting.currency), bdb::on(Currency.id)));

// The balance is a query (the ledger's law): Σ over the PLAIN SCALAR
// variable, grouped by the non-aggregated head columns; the fresh id is
// bound so identical postings never collapse before the fold.
inline constexpr auto Totals = bdb::query(Money).rule([](auto r) consteval {
	auto vars = r.vars(Posting);
	return r
	    .match(Posting,
	           {
	               .id = vars.id,
	               .account = vars.account,
	               .currency = vars.currency,
	               .minor = vars.minor,
	           })
	    .find(
	        {
	            .account = vars.account,
	            .currency = vars.currency,
	        },
	        bdb::sum<"total">(vars.minor));
});

namespace {

struct CaseResult {
	std::string name;
	bool passed;
};

/// The golden of one recipe: the fixtures file is one `rNN <64-hex>` line
/// per recipe (ts/test/cookbook.test.ts reads the same file).
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
	auto const dir = root / std::format("bumbledb-cookbook-r04-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t cash;
	std::uint64_t bank;
};

/// Two accounts, four postings: cash carries two USD postings (they must
/// FOLD, not collapse) and one EUR posting; bank carries one GBP posting.
auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto cash = tx.alloc(Account.id);
		if (!cash.has_value()) {
			return std::unexpected{std::move(cash).error()};
		}
		auto bank = tx.alloc(Account.id);
		if (!bank.has_value()) {
			return std::unexpected{std::move(bank).error()};
		}
		auto const posting = [&](std::uint64_t account, bdb::ref_to<Currency.id> currency,
		                         std::int64_t minor) -> std::expected<bool, bdb::Error> {
			return tx.alloc(Posting.id).and_then([&](std::uint64_t id) {
				return tx.insert(Posting, PostingRow{.id = id, .account = account, .currency = currency, .minor = minor});
			});
		};
		auto rows_land = tx.insert(Account, AccountRow{.id = *cash, .name = "cash"})
		                     .and_then([&](bool) {
			                     return tx.insert(Account, AccountRow{.id = *bank, .name = "bank"});
		                     })
		                     .and_then([&](bool) {
			                     return posting(*cash, Currency.Usd, 1250);
		                     })
		                     .and_then([&](bool) {
			                     return posting(*cash, Currency.Usd, 250);
		                     })
		                     .and_then([&](bool) {
			                     return posting(*cash, Currency.Eur, -40);
		                     })
		                     .and_then([&](bool) {
			                     return posting(*bank, Currency.Gbp, 7);
		                     });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(SeedIds{.cash = *cash, .bank = *bank});
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<SeedIds>>(*written).value;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r04") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r04 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r04 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Money);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Money", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r04 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto totals = db->prepare<Totals>();
	results.push_back(CaseResult{
	    .name = "totals (grouped scalar sum) prepares through the engine "
	            "validator",
	    .passed = totals.has_value(),
	});
	if (!totals.has_value()) {
		return;
	}

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "two accounts + four postings commit (handle -> row id "
	            "marshal)",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	// One answer row per (account, currency); the two USD postings fold.
	using Total = std::tuple<std::uint64_t, std::uint64_t, std::int64_t>;
	auto answers = db->execute(*totals, {});
	auto actual = std::vector<Total>{};
	if (answers.has_value()) {
		for (auto const& row : answers->rows()) {
			actual.emplace_back(row.account, row.currency, row.total);
		}
		std::ranges::sort(actual);
	}
	auto expected = std::vector<Total>{
	    {ids->cash, Currency.Usd.index, 1500},
	    {ids->cash, Currency.Eur.index, -40},
	    {ids->bank, Currency.Gbp.index, 7},
	};
	std::ranges::sort(expected);
	results.push_back(CaseResult{
	    .name = "totals answers Σ minor per (account, currency)",
	    .passed = answers.has_value() && actual == expected,
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r04_money <fixtures-file>");
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
