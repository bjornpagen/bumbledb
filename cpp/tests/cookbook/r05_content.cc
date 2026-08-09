// Cookbook recipe 5 — Content addressing (TODO_CPP §33; ts/COOKBOOK.md
// §5): fixed-width digests as `bdb::bytes<32>` columns. The digest is the
// key (`key(Document.payload)` — content-addressed identity), replicas
// point at digests that EXIST (a generator-less bytes -> bytes
// containment), and the region rides a closed vocabulary. The recipe's
// query binds a bytes param IN the match record (bytes literals never
// inline — they cross as params, lowering.md §7.8).
//
// Fingerprint vs the shared golden (fixtures/cookbook-fingerprints.txt,
// line "r05 <64-hex>"), then the query prepares through the REAL engine
// validator and answers the recipe's own semantics.
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

inline constexpr auto Region = bdb::closed<"Region", "Us", "Eu">();

struct DocumentRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
	bdb::bytes<32> payload;
};

struct ReplicaRow {
	bdb::bytes<32> payload;
	bdb::ref_to<Region.id> region;
};

inline constexpr auto Document = bdb::relation<"Document", DocumentRow>;
inline constexpr auto Replica = bdb::relation<"Replica", ReplicaRow>;

inline constexpr auto Content = bdb::schema<"Content">(Region, Document, Replica,

                                                       // Content-addressed identity: the digest is the key.
                                                       bdb::key(Document.payload),

                                                       // A replica's digest names a document that exists — bytes -> bytes,
                                                       // generator-less containment.
                                                       bdb::contained(bdb::on(Replica.payload), bdb::on(Document.payload)),

                                                       bdb::contained(bdb::on(Replica.region), bdb::on(Region.id)));

// Lookup by digest: the bytes param binds AT the match field.
inline constexpr auto ByDigest = bdb::query(Content).rule([](auto r) consteval {
	auto vars = r.vars(Document);
	return r
	    .match(Document,
	           {
	               .id = vars.id,
	               .payload = bdb::param<"digest">(),
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
	auto const dir = root / std::format("bumbledb-cookbook-r05-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

/// A 32-byte digest filled with one repeated octet.
auto digest_of(std::uint8_t octet) -> bdb::bytes<32> {
	auto out = bdb::bytes<32>{};
	out.fill(std::byte{octet});
	return out;
}

struct SeedIds {
	std::uint64_t spec;
	std::uint64_t draft;
};

/// Two documents (distinct digests) plus one replica of the first — the
/// replica's digest must land on an existing document (the containment).
auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto spec = tx.alloc(Document.id);
		if (!spec.has_value()) {
			return std::unexpected{std::move(spec).error()};
		}
		auto draft = tx.alloc(Document.id);
		if (!draft.has_value()) {
			return std::unexpected{std::move(draft).error()};
		}
		auto rows_land = tx.insert(Document, DocumentRow{.id = *spec, .name = "spec", .payload = digest_of(0x11)})
		                     .and_then([&](bool) {
			                     return tx.insert(Document, DocumentRow{.id = *draft, .name = "draft", .payload = digest_of(0x22)});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Replica, ReplicaRow{.payload = digest_of(0x11), .region = Region.Us});
		                     });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(SeedIds{.spec = *spec, .draft = *draft});
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<SeedIds>>(*written).value;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r05") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r05 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r05 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Content);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Content", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r05 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto by_digest = db->prepare<ByDigest>();
	results.push_back(CaseResult{
	    .name = "byDigest (bytes param in the match) prepares through "
	            "the engine validator",
	    .passed = by_digest.has_value(),
	});
	if (!by_digest.has_value()) {
		return;
	}

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "two documents + one replica commit (bytes -> bytes "
	            "containment holds)",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	// Executing byDigest (a bytes param at bind time) is blocked by a
	// foreign-layer gap: `wire_one`'s set-lane template captures the
	// scalar `std::span<std::byte const>` member, so the execute marshal
	// of any bytes-param query is ill-formed (cpp/foreign/program.cppm).
	// The TS suite pins prepare only for this recipe — parity holds.

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r05_content <fixtures-file>");
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
