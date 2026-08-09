import std;
import bumbledb;

inline constexpr auto Arm = bdb::closed<"Arm", "Busy", "Ooo">();

struct ClaimRow {
	std::uint64_t source;
	std::uint64_t person;
	bdb::ref_to<Arm.id> arm;
	bdb::interval<std::int64_t> span;
};

struct BusySpanRow {
	std::uint64_t person;
	bdb::interval<std::int64_t> span;
};

inline constexpr auto Claim = bdb::relation<"Claim", ClaimRow>;
inline constexpr auto BusySpan = bdb::relation<"BusySpan", BusySpanRow>;

inline constexpr auto Rollup = bdb::schema<"Rollup">(
    Arm, Claim, BusySpan,

    bdb::contained(bdb::on(Claim.arm), bdb::on(Arm.id)), bdb::key(Claim.source), bdb::key(Claim.person, Claim.span),

    bdb::key(BusySpan.person, BusySpan.span),

    bdb::contained(bdb::on(BusySpan.person, BusySpan.span), bdb::on(bdb::where(Claim, {.arm = Arm.Busy}), Claim.person, Claim.span)));

inline constexpr auto Deriving = bdb::query(Rollup).rule([](auto r) consteval {
	auto vars = r.vars(Claim);
	return r
	    .match(Claim,
	           {
	               .person = vars.person,
	               .arm = Arm.Busy,
	               .span = vars.span,
	           })
	    .find(
	        {
	            .person = vars.person,
	        },
	        bdb::pack<"packed">(vars.span));
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
	auto const dir = root / std::format("bumbledb-cookbook-r21-{:08x}{:08x}", device(), device());
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
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r21") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r21 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r21 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Rollup);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Rollup", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r21 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto deriving = db->prepare<Deriving>();
	results.push_back(CaseResult{
	    .name = "deriving (handle literal + pack) prepares through the "
	            "engine validator",
	    .passed = deriving.has_value(),
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r21_rollup <fixtures-file>");
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
