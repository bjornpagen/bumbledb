// Cookbook recipe 18 — Free time and coalescing (ts/COOKBOOK.md §18):
// `pack` is Snodgrass's coalesce as an aggregate — maximal disjoint
// segments per group, one answer per (group, segment). Coalescing is
// never a write rule: the engine stores the claims it was given, and the
// schema deliberately declares NO pointwise key on Claim — claims overlap
// freely and pack coalesces at read time (wanting them stored-disjoint is
// recipe 1's key instead).
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r18 <64-hex>"); busy (`r.pack(span)`, adjacent segments merge — the
// half-open law) and claimed (`r.sum(r.duration(span))`, overlaps
// double-count — often the wrong question) prepare AND answer the
// recipe's own semantics over overlapping claims the keyless relation
// admits.
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

struct PersonRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct ClaimRow {
	std::uint64_t person;
	bdb::interval<std::int64_t> span;
};

inline constexpr auto Person = bdb::relation<"Person", PersonRow>;
inline constexpr auto Claim = bdb::relation<"Claim", ClaimRow>;

inline constexpr auto FreeTime = bdb::schema<"FreeTime">(Person, Claim,

                                                         bdb::contained(bdb::on(Claim.person), bdb::on(Person.id))
                                                         // No pointwise key, on purpose: claims overlap freely and pack
                                                         // coalesces at read time.
);

// busy time, coalesced (adjacent segments merge — the half-open law).
inline constexpr auto Busy = bdb::query(FreeTime).rule([](auto r) consteval {
	auto vars = r.vars(Claim);
	return r
	    .match(Claim,
	           {
	               .person = vars.person,
	               .span = vars.span,
	           })
	    .find(
	        {
	            .person = vars.person,
	        },
	        bdb::pack<"packed">(vars.span));
});

// raw claimed time (overlaps double-count — often the wrong question).
inline constexpr auto Claimed = bdb::query(FreeTime).rule([](auto r) consteval {
	auto vars = r.vars(Claim);
	return r
	    .match(Claim,
	           {
	               .person = vars.person,
	               .span = vars.span,
	           })
	    .find(
	        {
	            .person = vars.person,
	        },
	        bdb::sum<"claimed">(r.duration(vars.span)));
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
	auto const dir = root / std::format("bumbledb-cookbook-r18-{:08x}{:08x}", device(), device());
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
};

/// Alice's claims OVERLAP and MEET — [0,10), [5,15), [20,30) — exactly
/// what the keyless relation admits; bob claims [0,5).
[[nodiscard]] auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto alice = tx.alloc(Person.id);
		if (!alice.has_value()) {
			return std::unexpected{std::move(alice).error()};
		}
		auto bob = tx.alloc(Person.id);
		if (!bob.has_value()) {
			return std::unexpected{std::move(bob).error()};
		}
		auto rows_land = tx.insert(Person, PersonRow{.id = *alice, .name = std::string{"alice"}})
		                     .and_then([&](bool) {
			                     return tx.insert(Person, PersonRow{.id = *bob, .name = std::string{"bob"}});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Claim, ClaimRow{.person = *alice, .span = bdb::interval<std::int64_t>::literal(0, 10)});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Claim, ClaimRow{.person = *alice, .span = bdb::interval<std::int64_t>::literal(5, 15)});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Claim, ClaimRow{.person = *alice, .span = bdb::interval<std::int64_t>::literal(20, 30)});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Claim, ClaimRow{.person = *bob, .span = bdb::interval<std::int64_t>::literal(0, 5)});
		                     });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(SeedIds{.alice = *alice, .bob = *bob});
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<SeedIds>>(*written).value;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r18") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r18 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r18 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), FreeTime);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits FreeTime", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r18 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "overlapping claims commit (keyless on purpose — the "
	            "engine stores the claims it was given)",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	auto busy = db->prepare<Busy>();
	auto claimed = db->prepare<Claimed>();
	results.push_back(CaseResult{
	    .name = "busy / claimed prepare through the engine validator",
	    .passed = busy.has_value() && claimed.has_value(),
	});
	if (!busy.has_value() || !claimed.has_value()) {
		return;
	}

	// pack: maximal disjoint segments per person — alice's [0,10) and
	// [5,15) overlap into [0,15); [20,30) stays its own segment (a gap is
	// never bridged); bob keeps [0,5).
	auto segments = db->execute(*busy, {}).transform([](bdb::Answers<Busy> answers) {
		auto rows = std::vector<std::pair<std::uint64_t, bdb::interval<std::int64_t>>>{};
		for (auto const& row : answers.rows()) {
			rows.emplace_back(row.person, row.packed);
		}
		std::ranges::sort(rows, [](auto const& left, auto const& right) {
			if (left.first != right.first) {
				return left.first < right.first;
			}
			return left.second.lo() < right.second.lo();
		});
		return rows;
	});
	auto expected_segments = std::vector<std::pair<std::uint64_t, bdb::interval<std::int64_t>>>{
	    {ids->alice, bdb::interval<std::int64_t>::literal(0, 15)},
	    {ids->alice, bdb::interval<std::int64_t>::literal(20, 30)},
	    {ids->bob, bdb::interval<std::int64_t>::literal(0, 5)},
	};
	std::ranges::sort(expected_segments, [](auto const& left, auto const& right) {
		if (left.first != right.first) {
			return left.first < right.first;
		}
		return left.second.lo() < right.second.lo();
	});
	results.push_back(CaseResult{
	    .name = "busy packs alice to {[0,15), [20,30)} and bob to "
	            "{[0,5)} (one answer per (person, maximal segment))",
	    .passed = segments.has_value() && *segments == expected_segments,
	});

	// sum(duration): the raw claimed time double-counts alice's overlap
	// (10 + 10 + 10 = 30 over 25 distinct points).
	auto totals = db->execute(*claimed, {}).transform([](bdb::Answers<Claimed> answers) {
		auto rows = std::vector<std::pair<std::uint64_t, std::uint64_t>>{};
		for (auto const& row : answers.rows()) {
			rows.emplace_back(row.person, row.claimed);
		}
		std::ranges::sort(rows, {}, [](auto const& row) {
			return row.first;
		});
		return rows;
	});
	auto expected_totals = std::vector<std::pair<std::uint64_t, std::uint64_t>>{{ids->alice, 30}, {ids->bob, 5}};
	std::ranges::sort(expected_totals, {}, [](auto const& row) {
		return row.first;
	});
	results.push_back(CaseResult{
	    .name = "claimed answers {(alice, 30), (bob, 5)} (overlaps "
	            "double-count — often the wrong question)",
	    .passed = totals.has_value() && *totals == expected_totals,
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r18_freetime <fixtures-file>");
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
