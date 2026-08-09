import std;
import bumbledb;

struct AccountRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct JournalEntryRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::int64_t at;
	std::string memo;
};

struct PostingRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::uint64_t entry;
	std::uint64_t account;
	std::int64_t minor;
};

inline constexpr auto Account = bdb::relation<"Account", AccountRow>;
inline constexpr auto JournalEntry = bdb::relation<"JournalEntry", JournalEntryRow>;
inline constexpr auto Posting = bdb::relation<"Posting", PostingRow>;

inline constexpr auto Ledger = bdb::schema<"Ledger">(Account, JournalEntry, Posting,

                                                     bdb::contained(bdb::on(Posting.entry), bdb::on(JournalEntry.id)),
                                                     bdb::contained(bdb::on(Posting.account), bdb::on(Account.id))
);

inline constexpr auto Balances = bdb::query(Ledger).rule([](auto r) consteval {
	auto vars = r.vars(Posting);
	return r
	    .match(Posting,
	           {
	               .id = vars.id,
	               .account = vars.account,
	               .minor = vars.minor,
	           })
	    .find(
	        {
	            .account = vars.account,
	        },
	        bdb::sum<"balance">(vars.minor));
});

inline constexpr auto DoubleEntry = bdb::query(Ledger).rule([](auto r) consteval {
	auto vars = r.vars(Posting);
	return r
	    .match(Posting,
	           {
	               .id = vars.id,
	               .entry = vars.entry,
	               .minor = vars.minor,
	           })
	    .find(
	        {
	            .entry = vars.entry,
	        },
	        bdb::sum<"balance">(vars.minor));
});

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
	auto const dir = root / std::format("bumbledb-cookbook-r19-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r19") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r19 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r19 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Ledger);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Ledger", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r19 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto balances = db->prepare<Balances>();
	results.push_back(CaseResult{
	    .name = "balances (sum over the scalar minor, fresh id bound) "
	            "prepares through the engine validator",
	    .passed = balances.has_value(),
	});

	auto double_entry = db->prepare<DoubleEntry>();
	results.push_back(CaseResult{
	    .name = "doubleEntry (the per-entry audit) prepares through the "
	            "engine validator",
	    .passed = double_entry.has_value(),
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r19_ledger <fixtures-file>");
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
