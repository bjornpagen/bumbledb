// Cookbook recipe 3 — 0..1 optional attributes (TODO_CPP §33;
// ts/COOKBOOK.md §3): optionality is a RELATION, never a nullable column.
// `MailingAddress` holds at most one row per business (the one-column
// key) and every address points at a real business (the one-way
// containment — deliberately NOT mirrored, so address-less businesses are
// legal). The recipe's query is the anti-join: businesses with NO mailing
// address, spelled as one negated EDB atom (`.not_match` — the C++ image
// of TS `not(MailingAddress, { business: b })`).
//
// Fingerprint vs the shared golden (fixtures/cookbook-fingerprints.txt,
// line "r03 <64-hex>"), then the query prepares through the REAL engine
// validator and answers the recipe's own semantics.
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

struct BusinessRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct MailingAddressRow {
	std::uint64_t business;
	std::string line;
	std::string city;
};

inline constexpr auto Business = bdb::relation<"Business", BusinessRow>;
inline constexpr auto MailingAddress = bdb::relation<"MailingAddress", MailingAddressRow>;

inline constexpr auto Optionality = bdb::schema<"Optionality">(Business, MailingAddress,

                                                               // 0..1: at most one address per business.
                                                               bdb::key(MailingAddress.business),

                                                               // Every address points at a real business — one-way on purpose (no
                                                               // mirror), so a business without an address is a legal state.
                                                               bdb::contained(bdb::on(MailingAddress.business), bdb::on(Business.id)));

// The anti-join: keep every business no MailingAddress fact extends. The
// negated atom binds nothing — `vars.id` is grounded by the positive
// Business atom (the safety rule).
inline constexpr auto Unaddressed = bdb::query(Optionality).rule([](auto r) consteval {
	auto vars = r.vars(Business);
	return r
	    .match(Business,
	           {
	               .id = vars.id,
	           })
	    .not_match(MailingAddress,
	               {
	                   .business = vars.id,
	               })
	    .find({
	        .id = vars.id,
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
	auto const dir = root / std::format("bumbledb-cookbook-r03-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t acme;
	std::uint64_t shell;
};

/// Two businesses; only acme carries the optional mailing address.
[[nodiscard]] auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto acme = tx.alloc(Business.id);
		if (!acme.has_value()) {
			return std::unexpected{std::move(acme).error()};
		}
		auto shell = tx.alloc(Business.id);
		if (!shell.has_value()) {
			return std::unexpected{std::move(shell).error()};
		}
		auto rows_land =
		    tx.insert(Business, BusinessRow{.id = *acme, .name = "acme"})
		        .and_then([&](bool) {
			        return tx.insert(Business, BusinessRow{.id = *shell, .name = "shell"});
		        })
		        .and_then([&](bool) {
			        return tx.insert(MailingAddress, MailingAddressRow{.business = *acme, .line = "1 Main St", .city = "Springfield"});
		        });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(SeedIds{.acme = *acme, .shell = *shell});
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<SeedIds>>(*written).value;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r03") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r03 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r03 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Optionality);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Optionality", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r03 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto unaddressed = db->prepare<Unaddressed>();
	results.push_back(CaseResult{
	    .name = "unaddressed (negated EDB atom) prepares through the "
	            "engine validator",
	    .passed = unaddressed.has_value(),
	});
	if (!unaddressed.has_value()) {
		return;
	}

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "two businesses + one optional address commit",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	// The anti-join keeps exactly the address-less business.
	auto answers = db->execute(*unaddressed, {});
	results.push_back(CaseResult{
	    .name = "unaddressed answers {shell} (the anti-join)",
	    .passed = answers.has_value() && answers->size() == 1 && answers->rows().front().id == ids->shell,
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r03_optionality <fixtures-file>");
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
