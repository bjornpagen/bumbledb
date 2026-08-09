import std;
import bumbledb;

inline constexpr auto Kind = bdb::closed<"Kind", "Deterministic", "CustomOperator">();

struct TaskRow {
	[[= bdb::fresh]] std::uint64_t id;

	bdb::ref_to<Kind.id> kind;
};

struct DeterministicGradingRow {
	std::uint64_t task;
	std::int64_t tolerance;
};

struct CustomOperatorGradingRow {
	std::uint64_t task;

	[[= bdb::named<"operator">]] std::string op;
};

inline constexpr auto Task = bdb::relation<"Task", TaskRow>;
inline constexpr auto DeterministicGrading = bdb::relation<"DeterministicGrading", DeterministicGradingRow>;
inline constexpr auto CustomOperatorGrading = bdb::relation<"CustomOperatorGrading", CustomOperatorGradingRow>;

inline constexpr auto Grading = bdb::schema<"Grading">(
    Kind, Task, DeterministicGrading, CustomOperatorGrading,

    bdb::contained(bdb::on(Task.kind), bdb::on(Kind.id)),

    bdb::key(DeterministicGrading.task), bdb::key(CustomOperatorGrading.task),

    bdb::mirrors(bdb::on(bdb::where(Task, {.kind = Kind.Deterministic}), Task.id), bdb::on(DeterministicGrading.task)),
    bdb::mirrors(bdb::on(bdb::where(Task, {.kind = Kind.CustomOperator}), Task.id), bdb::on(CustomOperatorGrading.task)));

namespace {

[[nodiscard]] constexpr auto graded_by(bdb::ref_to<Kind.id> kind) -> std::string_view {
	switch (kind.row) {
	case Kind.Deterministic.index:
		return "tolerance";
	case Kind.CustomOperator.index:
		return "operator";
	default:
		return "unreachable";
	}
}

static_assert(graded_by(Kind.Deterministic) == "tolerance");
static_assert(graded_by(Kind.CustomOperator) == "operator");

static_assert(bdb::ref_to<Kind.id>{Kind.Deterministic}.row == 0);
static_assert(bdb::ref_to<Kind.id>{Kind.CustomOperator}.row == 1);

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
	auto const dir = root / std::format("bumbledb-cookbook-r02-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r02_grading <fixtures-file>");
		return 1;
	}
	auto const fixtures = slurp(std::string_view{arguments[1]});
	if (!fixtures.has_value()) {
		std::println("FAIL: cannot read fixtures file {}", std::string_view{arguments[1]});
		return 1;
	}
	auto const golden = golden_of(*fixtures, "r02");
	if (!golden.has_value()) {
		std::println("FAIL: fixtures file carries no r02 line");
		return 1;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		std::println("FAIL: cannot create a temp store directory");
		return 1;
	}

	auto db = bdb::Db::ephemeral(dir->native(), Grading);
	if (!db.has_value()) {
		std::println("FAIL: Db::ephemeral rejected the Grading schema: {}", db.error().message());
		return 1;
	}
	auto const fingerprint = db->fingerprint();
	if (!fingerprint.has_value()) {
		std::println("FAIL: fingerprint readback failed: {}", fingerprint.error().message());
		return 1;
	}

	auto failures = 0;
	if (*fingerprint == *golden) {
		std::println("pass: r02 fingerprint matches the pinned golden");
	} else {
		std::println("FAIL: r02 fingerprint mismatch");
		std::println("  golden: {}", *golden);
		std::println("  actual: {}", *fingerprint);
		++failures;
	}

	if (graded_by(Kind.Deterministic) == "tolerance" && graded_by(Kind.CustomOperator) == "operator") {
		std::println("pass: gradedBy dispatches over the handle projection");
	} else {
		std::println("FAIL: gradedBy dispatch");
		++failures;
	}

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
	return failures == 0 ? 0 : 1;
}
