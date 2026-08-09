import std;
import bumbledb;

struct PersonRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct RepoRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct FollowsRow {
	std::uint64_t follower;
	std::uint64_t followee;
};

struct MaintainsRow {
	std::uint64_t person;
	std::uint64_t repo;
};

inline constexpr auto Person = bdb::relation<"Person", PersonRow>;
inline constexpr auto Repo = bdb::relation<"Repo", RepoRow>;
inline constexpr auto Follows = bdb::relation<"Follows", FollowsRow>;
inline constexpr auto Maintains = bdb::relation<"Maintains", MaintainsRow>;

inline constexpr auto Graph = bdb::schema<"Graph">(
    Person, Repo, Follows, Maintains,

    bdb::contained(bdb::on(Follows.follower), bdb::on(Person.id)), bdb::contained(bdb::on(Follows.followee), bdb::on(Person.id)),

    bdb::key(Follows.follower, Follows.followee),

    bdb::contained(bdb::on(Maintains.person), bdb::on(Person.id)), bdb::contained(bdb::on(Maintains.repo), bdb::on(Repo.id)),
    bdb::key(Maintains.person, Maintains.repo));

inline constexpr auto Mutual = bdb::query(Graph).rule([](auto r) consteval {
	auto vars = r.vars(Follows);
	return r
	    .match(Follows,
	           {
	               .follower = vars.follower,
	               .followee = vars.followee,
	           })
	    .match(Follows,
	           {
	               .follower = vars.followee,
	               .followee = vars.follower,
	           })
	    .where(bdb::lt(vars.follower, vars.followee))
	    .find({}, bdb::as<"a">(vars.follower), bdb::as<"b">(vars.followee));
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
	auto const dir = root / std::format("bumbledb-cookbook-r11-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t alice;
	std::uint64_t bob;
	std::uint64_t carol;
	std::uint64_t repo;
};

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
		auto rows_land = mint(Person.id, ids.alice)
		                     .and_then([&](bool) {
			                     return mint(Person.id, ids.bob);
		                     })
		                     .and_then([&](bool) {
			                     return mint(Person.id, ids.carol);
		                     })
		                     .and_then([&](bool) {
			                     return mint(Repo.id, ids.repo);
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Person, PersonRow{.id = ids.alice, .name = std::string{"alice"}});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Person, PersonRow{.id = ids.bob, .name = std::string{"bob"}});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Person, PersonRow{.id = ids.carol, .name = std::string{"carol"}});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Repo, RepoRow{.id = ids.repo, .name = std::string{"bdb"}});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Follows, FollowsRow{.follower = ids.alice, .followee = ids.bob});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Follows, FollowsRow{.follower = ids.bob, .followee = ids.alice});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Follows, FollowsRow{.follower = ids.alice, .followee = ids.carol});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Maintains, MaintainsRow{.person = ids.alice, .repo = ids.repo});
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
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r11") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r11 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r11 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Graph);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Graph", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r11 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "nodes and typed edges commit",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	auto dangling = db->write([&](bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::monostate>, bdb::Error> {
		auto landed = tx.insert(Follows, FollowsRow{.follower = ids->alice, .followee = 999'999});
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
	results.push_back(CaseResult{
	    .name = "a Follows edge outside \"Person.id\" is commit-rejected "
	            "(the endpoint containment types the edge)",
	    .passed =
	        !dangling.has_value() && dangling.error().kind() == bdb::ErrorKind::CommitRejected && !dangling.error().violations().empty(),
	});

	auto mutual = db->prepare<Mutual>();
	results.push_back(CaseResult{
	    .name = "mutual prepares through the engine validator",
	    .passed = mutual.has_value(),
	});
	if (!mutual.has_value()) {
		return;
	}

	auto pairs = db->execute(*mutual, {});
	auto const low = std::min(ids->alice, ids->bob);
	auto const high = std::max(ids->alice, ids->bob);
	results.push_back(CaseResult{
	    .name = "mutual answers exactly {(alice, bob)}, each pair once",
	    .passed = pairs.has_value() && pairs->size() == 1 && pairs->rows().front().a == low && pairs->rows().front().b == high,
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r11_graph <fixtures-file>");
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
