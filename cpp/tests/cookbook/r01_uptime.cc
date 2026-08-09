// Cookbook recipe 1 — Uptime (TODO_CPP §33, §39; ts/COOKBOOK.md §1): the
// cross-host parity gate. The §39 theory is built through the REAL
// bdb::schema<> elaborator, lowered through the C ABI SchemaSpec path,
// admitted by the engine, and the store's fingerprint readback must equal
// the host-neutral golden pinned at the repository root
// (fixtures/cookbook-fingerprints.txt, line "r01 <64-hex>") — the same
// line the TypeScript suite asserts. Identical bytes or the recipe is
// wrong (lowering.md §6–§7).
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

// TODO_CPP §39 — the first-slice rows, spelled exactly as specified.
struct ServiceRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct OutageRow {
	std::uint64_t service;
	bdb::interval<std::int64_t> window;
};

inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;
inline constexpr auto Outage = bdb::relation<"Outage", OutageRow>;

inline constexpr auto Uptime = bdb::schema<"Uptime">(Service, Outage,

                                                     bdb::contained(bdb::on(Outage.service), bdb::on(Service.id)),

                                                     bdb::key(Outage.service, Outage.window));

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
	auto const dir = root / std::format("bumbledb-cookbook-r01-{:08x}{:08x}", device(), device());
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
		std::println("FAIL: usage: r01_uptime <fixtures-file>");
		return 1;
	}
	auto const fixtures = slurp(std::string_view{arguments[1]});
	if (!fixtures.has_value()) {
		std::println("FAIL: cannot read fixtures file {}", std::string_view{arguments[1]});
		return 1;
	}
	auto const golden = golden_of(*fixtures, "r01");
	if (!golden.has_value()) {
		std::println("FAIL: fixtures file carries no r01 line");
		return 1;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		std::println("FAIL: cannot create a temp store directory");
		return 1;
	}

	auto db = bdb::Db::ephemeral(dir->native(), Uptime);
	if (!db.has_value()) {
		std::println("FAIL: Db::ephemeral rejected the Uptime schema: {}", db.error().message());
		return 1;
	}
	auto const fingerprint = db->fingerprint();
	if (!fingerprint.has_value()) {
		std::println("FAIL: fingerprint readback failed: {}", fingerprint.error().message());
		return 1;
	}

	auto failures = 0;
	if (*fingerprint == *golden) {
		std::println("pass: r01 fingerprint matches the pinned golden");
	} else {
		std::println("FAIL: r01 fingerprint mismatch");
		std::println("  golden: {}", *golden);
		std::println("  actual: {}", *fingerprint);
		++failures;
	}

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
	return failures == 0 ? 0 : 1;
}
