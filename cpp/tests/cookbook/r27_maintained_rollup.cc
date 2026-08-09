// Cookbook recipe 27 — Derived facts, maintained (ts/COOKBOOK.md §27): a
// stored rollup is an ORDINARY relation with an ordinary soundness
// statement. `pack` derives the maximal busy spans on the maintenance
// snapshot, while the ψ-selected coverage containment prevents any
// stored `BusySpan` point that has no Busy claim behind it. That is
// soundness, not a refresh theorem: a missing span remains representable
// until the host maintenance loop fills it.
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r27 <64-hex>" — the SAME hex as r21, the theories coincide); the
// deriving query (the closed-handle literal + pack fold) prepares AND
// answers the coalesced spans; the derived rollup row commits (sound —
// the union of Busy claims covers it); a rollup row with no Busy claim
// behind it is commit-rejected.
//
// argv[1] = the fixtures file path (passed by add_test).
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

inline constexpr auto MaintainedRollup = bdb::schema<"MaintainedRollup">(
    Arm, Claim, BusySpan,

    bdb::contained(bdb::on(Claim.arm), bdb::on(Arm.id)), bdb::key(Claim.source), bdb::key(Claim.person, Claim.span),
    bdb::key(BusySpan.person, BusySpan.span),

    // Soundness: every stored BusySpan point has a Busy claim behind it.
    bdb::contained(bdb::on(BusySpan.person, BusySpan.span), bdb::on(bdb::where(Claim, {.arm = Arm.Busy}), Claim.person, Claim.span)));

// Derive the desired rollup on the maintenance snapshot: one answer row
// per (person, maximal Busy segment).
inline constexpr auto Deriving = bdb::query(MaintainedRollup).rule([](auto r) consteval {
	auto vars = r.vars(Claim);
	return r
	    .match(Claim,
	           {
	               .source = vars.source,
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
	auto const dir = root / std::format("bumbledb-cookbook-r27-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

/// Two persons' claims: person 1 is Busy [10,20) and [20,30) (touching —
/// pack coalesces them) plus Ooo [40,50) (the ψ selection excludes it);
/// person 2 is Busy [5,10). Sources are the claims' own key.
[[nodiscard]] auto seed(bdb::Db& db) -> bool {
	using Decision = bdb::WriteDecision<std::monostate, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto const claim = [&](std::uint64_t source, std::uint64_t person, auto arm,
		                       bdb::interval<std::int64_t> span) -> std::expected<bool, bdb::Error> {
			return tx.insert(Claim, ClaimRow{.source = source, .person = person, .arm = arm, .span = span});
		};
		auto rows_land = claim(1, 1, Arm.Busy, bdb::interval<std::int64_t>::literal(10, 20))
		                     .and_then([&](bool) {
			                     return claim(2, 1, Arm.Busy, bdb::interval<std::int64_t>::literal(20, 30));
		                     })
		                     .and_then([&](bool) {
			                     return claim(3, 1, Arm.Ooo, bdb::interval<std::int64_t>::literal(40, 50));
		                     })
		                     .and_then([&](bool) {
			                     return claim(4, 2, Arm.Busy, bdb::interval<std::int64_t>::literal(5, 10));
		                     });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit();
	});
	return written.has_value() && std::holds_alternative<bdb::Committed<std::monostate>>(*written);
}

[[nodiscard]] auto store_span(bdb::Db& db, std::uint64_t person, bdb::interval<std::int64_t> span)
    -> std::expected<bdb::WriteOutcome<std::monostate, std::monostate>, bdb::Error> {
	return db.write([&](bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::monostate>, bdb::Error> {
		auto landed = tx.insert(BusySpan, BusySpanRow{.person = person, .span = span});
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r27") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r27 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r27 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), MaintainedRollup);
	if (!db.has_value()) {
		results.push_back(CaseResult{.name = "Db::ephemeral admits MaintainedRollup", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r27 fingerprint matches the pinned golden (the same hex "
	            "as r21 — the theories coincide)",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto deriving = db->prepare<Deriving>();
	results.push_back(CaseResult{
	    .name = "deriving prepares through the engine validator (the "
	            "closed-handle literal + the pack fold)",
	    .passed = deriving.has_value(),
	});
	if (!deriving.has_value()) {
		return;
	}

	results.push_back(CaseResult{
	    .name = "the claims land (Busy pair, Ooo decoy, second person)",
	    .passed = seed(*db),
	});

	// The derivation on the maintenance snapshot: person 1's touching
	// Busy spans coalesce to ONE [10,30) block (the Ooo claim never
	// enters the fold); person 2 keeps [5,10).
	auto derived = db->execute(*deriving, {});
	auto rows = std::vector<std::pair<std::uint64_t, bdb::interval<std::int64_t>>>{};
	if (derived.has_value()) {
		for (auto const& row : derived->rows()) {
			rows.emplace_back(row.person, row.packed);
		}
		std::ranges::sort(rows, {}, [](auto const& row) {
			return row.first;
		});
	}
	results.push_back(CaseResult{
	    .name = "deriving answers {(1, [10,30)), (2, [5,10))} — pack "
	            "coalesces, the ψ arm filters",
	    .passed = derived.has_value() && rows.size() == 2 && rows[0].first == 1 &&
	              rows[0].second == bdb::interval<std::int64_t>::literal(10, 30) && rows[1].first == 2 &&
	              rows[1].second == bdb::interval<std::int64_t>::literal(5, 10),
	});

	// The maintenance write: the derived span is SOUND — every point of
	// [10,30) has a Busy claim behind it (the union of the two claims).
	auto sound = store_span(*db, 1, bdb::interval<std::int64_t>::literal(10, 30));
	results.push_back(CaseResult{
	    .name = "the derived rollup row commits (containment proves the "
	            "surviving span sound)",
	    .passed = sound.has_value() && std::holds_alternative<bdb::Committed<std::monostate>>(*sound),
	});

	// A span with no Busy claim behind it — person 1's [40,50) is Ooo,
	// not Busy — violates the soundness containment.
	auto unsound = store_span(*db, 1, bdb::interval<std::int64_t>::literal(40, 50));
	results.push_back(CaseResult{
	    .name = "a rollup row with no Busy claim behind it is "
	            "commit-rejected (soundness, not freshness)",
	    .passed = !unsound.has_value() && unsound.error().kind() == bdb::ErrorKind::CommitRejected && !unsound.error().violations().empty(),
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r27_maintained_rollup <fixtures-file>");
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
