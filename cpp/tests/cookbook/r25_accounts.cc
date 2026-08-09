import std;
import bumbledb;

struct AccountRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct AccountParentRow {
	std::uint64_t child;
	std::uint64_t parent;
};

struct PostingRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::uint64_t account;
	std::int64_t minor;
};

inline constexpr auto Account = bdb::relation<"Account", AccountRow>;
inline constexpr auto AccountParent = bdb::relation<"AccountParent", AccountParentRow>;
inline constexpr auto Posting = bdb::relation<"Posting", PostingRow>;

inline constexpr auto Accounts = bdb::schema<"Accounts">(
    Account, AccountParent, Posting,

    bdb::key(AccountParent.child), bdb::contained(bdb::on(AccountParent.child), bdb::on(Account.id)),
    bdb::contained(bdb::on(AccountParent.parent), bdb::on(Account.id)), bdb::contained(bdb::on(Posting.account), bdb::on(Account.id)));

inline constexpr auto FrontierStep = bdb::query(Accounts).rule([](auto r) consteval {
	auto vars = r.vars(AccountParent);
	return r
	    .match(AccountParent,
	           {
	               .child = vars.child,
	               .parent = bdb::set_param<"frontier">(),
	           })
	    .find({}, bdb::as<"c">(vars.child));
});

inline constexpr auto SubtreeRollup = bdb::query(Accounts).rule([](auto r) consteval {
	auto vars = r.vars(Posting);
	return r
	    .match(Posting,
	           {
	               .id = vars.id,
	               .account = bdb::set_param<"subtree">(),
	               .minor = vars.minor,
	           })
	    .find({}, bdb::sum<"total">(vars.minor));
});

inline constexpr auto NativeRollup = bdb::program(
    Accounts,
    bdb::rec<"sub">(
        [](auto r) consteval {
	        auto vars = r.vars(Account);
	        return r.match(Account, {.id = vars.id}).where(bdb::eq(vars.id, bdb::param<"root">())).find({}, bdb::as<"a">(vars.id));
        },
        [](auto r) consteval {
	        auto vars = r.vars(AccountParent);
	        return r.match(AccountParent, {.child = vars.child, .parent = vars.parent})
	            .idb(bdb::pred<"sub">, bdb::bind<"a">(vars.parent))
	            .find({}, bdb::as<"a">(vars.child));
        }),
    bdb::output([](auto r) consteval {
	    auto vars = r.vars(Posting);
	    return r
	        .match(Posting,
	               {
	                   .id = vars.id,
	                   .account = vars.account,
	                   .minor = vars.minor,
	               })
	        .idb(bdb::pred<"sub">, bdb::bind<"a">(vars.account))
	        .find({}, bdb::sum<"total">(vars.minor));
    }));

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
	auto const dir = root / std::format("bumbledb-cookbook-r25-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t root;
	std::uint64_t ops;
	std::uint64_t eng;
	std::uint64_t fe;
};

[[nodiscard]] auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto ids = SeedIds{};
		auto const account = [&](std::uint64_t& out, std::string name) -> std::expected<bool, bdb::Error> {
			return tx.alloc(Account.id).and_then([&](std::uint64_t minted) -> std::expected<bool, bdb::Error> {
				out = minted;
				return tx.insert(Account, AccountRow{.id = minted, .name = std::move(name)});
			});
		};
		auto const posting = [&](std::uint64_t into, std::int64_t minor) -> std::expected<bool, bdb::Error> {
			return tx.alloc(Posting.id).and_then([&](std::uint64_t minted) -> std::expected<bool, bdb::Error> {
				return tx.insert(Posting, PostingRow{.id = minted, .account = into, .minor = minor});
			});
		};
		auto rows_land = account(ids.root, std::string{"root"})
		                     .and_then([&](bool) {
			                     return account(ids.ops, std::string{"ops"});
		                     })
		                     .and_then([&](bool) {
			                     return account(ids.eng, std::string{"eng"});
		                     })
		                     .and_then([&](bool) {
			                     return account(ids.fe, std::string{"fe"});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(AccountParent, AccountParentRow{.child = ids.ops, .parent = ids.root});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(AccountParent, AccountParentRow{.child = ids.eng, .parent = ids.root});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(AccountParent, AccountParentRow{.child = ids.fe, .parent = ids.eng});
		                     })
		                     .and_then([&](bool) {
			                     return posting(ids.root, 5);
		                     })
		                     .and_then([&](bool) {
			                     return posting(ids.ops, 10);
		                     })
		                     .and_then([&](bool) {
			                     return posting(ids.eng, 20);
		                     })
		                     .and_then([&](bool) {
			                     return posting(ids.eng, 20);
		                     })
		                     .and_then([&](bool) {
			                     return posting(ids.fe, 30);
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

template<auto Step, auto Rollup>
[[nodiscard]] auto host_rollup(bdb::Db& db, bdb::Prepared<Step>& step, bdb::Prepared<Rollup>& rollup, std::uint64_t root) -> std::optional<std::int64_t> {
	auto subtree = std::vector<std::uint64_t>{root};
	auto frontier = std::vector<std::uint64_t>{root};
	while (!frontier.empty()) {
		auto next = db.execute(step, {.frontier = std::span<std::uint64_t const>{frontier}});
		if (!next.has_value()) {
			return std::nullopt;
		}
		auto fresh = std::vector<std::uint64_t>{};
		for (auto const& row : next->rows()) {
			if (!std::ranges::contains(subtree, row.c)) {
				fresh.push_back(row.c);
			}
		}
		for (auto const c : fresh) {
			subtree.push_back(c);
		}
		frontier = std::move(fresh);
	}
	auto folded = db.execute(rollup, {.subtree = std::span<std::uint64_t const>{subtree}});
	if (!folded.has_value() || folded->size() != 1) {
		return std::nullopt;
	}
	return folded->rows().front().total;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r25") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r25 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r25 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Accounts);
	if (!db.has_value()) {
		results.push_back(CaseResult{.name = "Db::ephemeral admits Accounts", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r25 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto step = db->prepare<FrontierStep>();
	auto rollup = db->prepare<SubtreeRollup>();
	auto native = db->prepare<NativeRollup>();
	results.push_back(CaseResult{
	    .name = "frontierStep / subtreeRollup / nativeRollup all prepare "
	            "through the engine validator (∈-set probe, ∈-set + sum "
	            "fold, fold over a finished lower stratum)",
	    .passed = step.has_value() && rollup.has_value() && native.has_value(),
	});
	if (!step.has_value() || !rollup.has_value() || !native.has_value()) {
		return;
	}

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "the chart of accounts lands (alloc + insert, equal "
	            "postings included)",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	auto const from_root = host_rollup<FrontierStep, SubtreeRollup>(*db, *step, *rollup, ids->root);
	auto native_root = db->execute(*native, {.root = ids->root});
	results.push_back(CaseResult{
	    .name = "root's rollup: host composition and native program both "
	            "answer 85 (equal postings both count)",
	    .passed = from_root.has_value() && *from_root == 85 && native_root.has_value() && native_root->size() == 1 &&
	              native_root->rows().front().total == 85,
	});

	auto const from_eng = host_rollup<FrontierStep, SubtreeRollup>(*db, *step, *rollup, ids->eng);
	auto native_eng = db->execute(*native, {.root = ids->eng});
	results.push_back(CaseResult{
	    .name = "eng's rollup: host composition and native program both "
	            "answer 70",
	    .passed = from_eng.has_value() && *from_eng == 70 && native_eng.has_value() && native_eng->size() == 1 &&
	              native_eng->rows().front().total == 70,
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r25_accounts <fixtures-file>");
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
