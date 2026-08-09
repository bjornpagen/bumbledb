// Cookbook recipe 23 — The anti-recipes: five gravestones
// (ts/COOKBOOK.md §23). What NOT to model — each gravestone names
// unsupported vocabulary and its representable replacement; the block's
// relations are the REPLACEMENTS, compiled:
//
//   - successor pointers (a `next` column)  -> the ordering triple (Step)
//   - floats for scores/rates/money         -> fixed-point i64 bps (Score)
//   - conditional keys ("at most one active
//     run per student" as an FD)            -> the relation split, whose
//                                             ordinary key IS the
//                                             invariant (ActiveRun)
//   - clip-at-query intervals               -> split at write (Usage)
//   - uuid keys (identity + clash-avoidance
//     + clock in one lie)                   -> fresh + an explicit i64
//                                             time column (Event)
//
// The refused spellings are compile-time walls in this dialect (no float
// field type exists; keys are unconditional projections) — the TS test
// admits only the replacements, and so does this one. Gate: the engine
// fingerprint equals the shared golden (fixtures line "r23 <64-hex>").
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

// REPLACEMENT for successor pointers: the ordering triple (recipe 9).
struct StepRow {
	std::uint64_t flow;
	std::uint64_t pos;
	std::string action;
};

// REPLACEMENT for floats: fixed-point i64 — basis points (recipe 4).
struct ScoreRow {
	std::uint64_t subject;
	std::int64_t bps;
};

// REPLACEMENT for conditional keys: the relation split, whose ordinary
// key IS the invariant (recipe 13's arm shape).
struct ActiveRunRow {
	std::uint64_t student;
	std::uint64_t run;
};

// REPLACEMENT for clip-at-query intervals: split at write (recipe 17).
struct UsageRow {
	std::uint64_t meter;
	std::uint64_t period;
	bdb::interval<std::int64_t> used;
};

// REPLACEMENT for uuid keys: fresh (minted identity) + an explicit i64
// time column.
struct EventRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::int64_t at;
};

inline constexpr auto Step = bdb::relation<"Step", StepRow>;
inline constexpr auto Score = bdb::relation<"Score", ScoreRow>;
inline constexpr auto ActiveRun = bdb::relation<"ActiveRun", ActiveRunRow>;
inline constexpr auto Usage = bdb::relation<"Usage", UsageRow>;
inline constexpr auto Event = bdb::relation<"Event", EventRow>;

inline constexpr auto Gravestones = bdb::schema<"Gravestones">(Step, Score, ActiveRun, Usage, Event,

                                                               bdb::key(Step.flow, Step.pos), bdb::key(Score.subject),
                                                               bdb::key(ActiveRun.student), bdb::key(Usage.meter, Usage.used));

namespace {

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
	auto const dir = root / std::format("bumbledb-cookbook-r23-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r23_gravestones <fixtures-file>");
		return 1;
	}
	auto const fixtures = slurp(std::string_view{arguments[1]});
	if (!fixtures.has_value()) {
		std::println("FAIL: cannot read fixtures file {}", std::string_view{arguments[1]});
		return 1;
	}
	auto const golden = golden_of(*fixtures, "r23");
	if (!golden.has_value()) {
		std::println("FAIL: fixtures file carries no r23 line");
		return 1;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		std::println("FAIL: cannot create a temp store directory");
		return 1;
	}

	auto db = bdb::Db::ephemeral(dir->native(), Gravestones);
	if (!db.has_value()) {
		std::println("FAIL: Db::ephemeral rejected the Gravestones schema: {}", db.error().message());
		return 1;
	}
	auto const fingerprint = db->fingerprint();
	if (!fingerprint.has_value()) {
		std::println("FAIL: fingerprint readback failed: {}", fingerprint.error().message());
		return 1;
	}

	auto failures = 0;
	if (*fingerprint == *golden) {
		std::println("pass: r23 fingerprint matches the pinned golden");
	} else {
		std::println("FAIL: r23 fingerprint mismatch");
		std::println("  golden: {}", *golden);
		std::println("  actual: {}", *fingerprint);
		++failures;
	}

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
	return failures == 0 ? 0 : 1;
}
