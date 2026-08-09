// Cookbook recipe 7 — The classification (TODO_CPP §8, §33;
// ts/COOKBOOK.md §7): the payload-tier closed vocabulary. Payload columns
// state what each word MEANS next to the word; ψ reads the payload on
// both sides:
//
//   schema — contained(Certificate.kind ⊆ Kind.where({mastered:true}).id)
//            lowers the ψ selection AS-IS (the ENGINE folds the member
//            set at validate);
//   query  — the closed relation is MATCHABLE like any relation:
//            match(Kind, {id: k, mastered: true}) with a bool payload
//            literal.
//
// Plus the typed axiom readback (`Kind.axioms.DirectPass.rank`) driving
// the record-table dispatch idiom. Fingerprint vs the shared golden;
// the query prepares through the real validator and answers the recipe's
// own semantics.
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

struct KindPayload {
	bool mastered;
	std::uint64_t rank;
};

inline constexpr auto Kind = bdb::closed<"Kind", KindPayload>(bdb::member<"DirectPass">(KindPayload{.mastered = true, .rank = 30}),
                                                              bdb::member<"JudgedPass">(KindPayload{.mastered = true, .rank = 20}),
                                                              bdb::member<"Failed">(KindPayload{.mastered = false, .rank = 10}));

struct AttemptRow {
	[[= bdb::fresh]] std::uint64_t id;

	bdb::ref_to<Kind.id> kind;
};

struct CertificateRow {
	std::uint64_t attempt;
	bdb::ref_to<Kind.id> kind;
};

inline constexpr auto Attempt = bdb::relation<"Attempt", AttemptRow>;
inline constexpr auto Certificate = bdb::relation<"Certificate", CertificateRow>;

inline constexpr auto Review =
    bdb::schema<"Review">(Kind, Attempt, Certificate,

                          bdb::contained(bdb::on(Attempt.kind), bdb::on(Kind.id)), bdb::key(Certificate.attempt),
                          bdb::contained(bdb::on(Certificate.attempt), bdb::on(Attempt.id)),

                          // ψ reads the payload: certificates carry mastered kinds only — the
                          // selected TARGET lowers pass-through; the engine compiles the member
                          // set {DirectPass, JudgedPass} at validate.
                          bdb::contained(bdb::on(Certificate.kind), bdb::on(bdb::where(Kind, {.mastered = true}), Kind.id)));

// The classification read duplicates no flag onto Attempt — ψ walks the
// vocabulary's payload in the query too: the closed relation is a query
// atom with a bool payload literal.
inline constexpr auto MasteredAttempts = bdb::query(Review).rule([](auto r) consteval {
	auto vars = r.vars(Attempt);
	return r
	    .match(Attempt,
	           {
	               .id = vars.id,
	               .kind = vars.kind,
	           })
	    .match(Kind,
	           {
	               .id = vars.kind,
	               .mastered = true,
	           })
	    .find({
	        .id = vars.id,
	    });
});

namespace {

// The typed axiom readback: the sealed rows, read off the facade value.
static_assert(Kind.axioms.DirectPass.mastered);
static_assert(Kind.axioms.DirectPass.rank == 30);
static_assert(Kind.axioms.JudgedPass.mastered);
static_assert(Kind.axioms.JudgedPass.rank == 20);
static_assert(!Kind.axioms.Failed.mastered);
static_assert(Kind.axioms.Failed.rank == 10);

// The record-table dispatch idiom (the cookbook's `labels`): one entry
// per handle, each reading its sealed axiom row off the typed readback;
// the switch over the handle projection is total over the roster.
[[nodiscard]] auto label(bdb::ref_to<Kind.id> kind) -> std::string {
	switch (kind.row) {
	case Kind.DirectPass.index:
		return std::format("mastered, rank {}", Kind.axioms.DirectPass.rank);
	case Kind.JudgedPass.index:
		return std::format("mastered, rank {}", Kind.axioms.JudgedPass.rank);
	case Kind.Failed.index:
		return "not mastered";
	default:
		return "unreachable";
	}
}

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
	auto const dir = root / std::format("bumbledb-cookbook-r07-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t direct;
	std::uint64_t judged;
	std::uint64_t failed;
};

/// One attempt per kind, plus a certificate for the DirectPass attempt
/// (the ψ containment admits mastered kinds — the commit proves it).
[[nodiscard]] auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto direct = tx.alloc(Attempt.id);
		if (!direct.has_value()) {
			return std::unexpected{std::move(direct).error()};
		}
		auto judged = tx.alloc(Attempt.id);
		if (!judged.has_value()) {
			return std::unexpected{std::move(judged).error()};
		}
		auto failed = tx.alloc(Attempt.id);
		if (!failed.has_value()) {
			return std::unexpected{std::move(failed).error()};
		}
		auto rows_land = tx.insert(Attempt, AttemptRow{.id = *direct, .kind = Kind.DirectPass})
		                     .and_then([&](bool) {
			                     return tx.insert(Attempt, AttemptRow{.id = *judged, .kind = Kind.JudgedPass});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Attempt, AttemptRow{.id = *failed, .kind = Kind.Failed});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Certificate, CertificateRow{.attempt = *direct, .kind = Kind.DirectPass});
		                     });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(SeedIds{.direct = *direct, .judged = *judged, .failed = *failed});
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<SeedIds>>(*written).value;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r07") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r07 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r07 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Review);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Review", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r07 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	// The record-table dispatch, off the sealed axioms.
	results.push_back(CaseResult{
	    .name = "label reads the typed Kind.axioms readback",
	    .passed = label(Kind.DirectPass) == "mastered, rank 30" && label(Kind.JudgedPass) == "mastered, rank 20" &&
	              label(Kind.Failed) == "not mastered",
	});

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "attempts + a mastered certificate commit (the ψ "
	            "containment admits DirectPass)",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	auto mastered = db->prepare<MasteredAttempts>();
	results.push_back(CaseResult{
	    .name = "masteredAttempts (a closed atom with a bool payload "
	            "literal) prepares",
	    .passed = mastered.has_value(),
	});
	if (!mastered.has_value()) {
		return;
	}

	auto answered = db->read([&](bdb::Snapshot& snap) -> std::expected<std::vector<std::uint64_t>, bdb::Error> {
		return snap.execute(*mastered, {}).transform([](bdb::Answers<MasteredAttempts> answers) {
			auto ids_out = std::vector<std::uint64_t>{};
			for (auto const& row : answers.rows()) {
				ids_out.push_back(row.id);
			}
			std::ranges::sort(ids_out);
			return ids_out;
		});
	});
	auto expected = std::vector{ids->direct, ids->judged};
	std::ranges::sort(expected);
	results.push_back(CaseResult{
	    .name = "masteredAttempts answers {direct, judged}",
	    .passed = answered.has_value() && *answered == expected,
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r07_review <fixtures-file>");
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
