import std;
import bumbledb;

struct PolicyRow {
	[[= bdb::fresh]] std::uint64_t id;

	bdb::interval<std::int64_t> live;
};

struct VersionRow {
	std::uint64_t policy;
	bdb::interval<std::int64_t> valid;
};

inline constexpr auto Policy = bdb::relation<"Policy", PolicyRow>;
inline constexpr auto Version = bdb::relation<"Version", VersionRow>;

inline constexpr auto ExactPartition =
    bdb::schema<"ExactPartition">(Policy, Version,

                                  bdb::contained(bdb::on(Version.policy), bdb::on(Policy.id)),

                                  bdb::key(Version.policy, Version.valid), bdb::key(Policy.id, Policy.live),

                                  bdb::contained(bdb::on(Policy.id, Policy.live), bdb::on(Version.policy, Version.valid)),
                                  bdb::contained(bdb::on(Version.policy, Version.valid), bdb::on(Policy.id, Policy.live)));

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
	auto const dir = root / std::format("bumbledb-cookbook-r26-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

[[nodiscard]] auto attempt(bdb::Db& db, std::vector<bdb::interval<std::int64_t>> const& versions)
    -> std::expected<bdb::WriteOutcome<std::monostate, std::monostate>, bdb::Error> {
	return db.write([&](bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::monostate>, bdb::Error> {
		auto landed = tx.alloc(Policy.id).and_then([&](std::uint64_t policy) -> std::expected<bool, bdb::Error> {
			auto row = tx.insert(Policy, PolicyRow{.id = policy, .live = bdb::interval<std::int64_t>::literal(0, 10)});
			for (auto const valid : versions) {
				row = std::move(row).and_then([&](bool) {
					return tx.insert(Version, VersionRow{.policy = policy, .valid = valid});
				});
			}
			return row;
		});
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r26") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r26 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r26 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), ExactPartition);
	if (!db.has_value()) {
		results.push_back(CaseResult{.name = "Db::ephemeral admits ExactPartition", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r26 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto exact = attempt(*db, {bdb::interval<std::int64_t>::literal(0, 5), bdb::interval<std::int64_t>::literal(5, 10)});
	results.push_back(CaseResult{
	    .name = "the exact partition commits (touching half-open "
	            "segments tile the policy span)",
	    .passed = exact.has_value() && std::holds_alternative<bdb::Committed<std::monostate>>(*exact),
	});

	auto gapped = attempt(*db, {bdb::interval<std::int64_t>::literal(0, 4), bdb::interval<std::int64_t>::literal(5, 10)});
	results.push_back(CaseResult{
	    .name = "a gap in the versions is commit-rejected (forward "
	            "coverage forbids gaps)",
	    .passed = !gapped.has_value() && gapped.error().kind() == bdb::ErrorKind::CommitRejected && !gapped.error().violations().empty(),
	});

	auto overhang = attempt(*db, {bdb::interval<std::int64_t>::literal(0, 12)});
	results.push_back(CaseResult{
	    .name = "a version overhang is commit-rejected (reverse coverage "
	            "forbids overhang)",
	    .passed =
	        !overhang.has_value() && overhang.error().kind() == bdb::ErrorKind::CommitRejected && !overhang.error().violations().empty(),
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r26_exact_partition <fixtures-file>");
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
