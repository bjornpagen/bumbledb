import std;
import bumbledb;

struct GrpRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string label;
};

struct CourseRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::uint64_t grp;
	std::string title;
};

inline constexpr auto Grp = bdb::relation<"Grp", GrpRow>;
inline constexpr auto Course = bdb::relation<"Course", CourseRow>;

inline constexpr auto course_grp_key = bdb::key(Course.grp);

inline constexpr auto KeyedRead = bdb::schema<"KeyedRead">(Grp, Course,

                                                           bdb::contained(bdb::on(Course.grp), bdb::on(Grp.id)),

                                                           course_grp_key);

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
	auto const dir = root / std::format("bumbledb-cookbook-r30-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

[[nodiscard]] auto cell_is_u64(bdb::RowSet const& rows, bdb::Cell at, std::uint64_t want) -> bool {
	auto const cell = rows.cell(at);
	return cell.has_value() && std::holds_alternative<std::uint64_t>(*cell) && std::get<std::uint64_t>(*cell) == want;
}

[[nodiscard]] auto cell_is_text(bdb::RowSet const& rows, bdb::Cell at, std::string_view want) -> bool {
	auto const cell = rows.cell(at);
	return cell.has_value() && std::holds_alternative<std::string>(*cell) && std::get<std::string>(*cell) == want;
}

struct SeedIds {
	std::uint64_t grp;
	std::uint64_t course;
};

[[nodiscard]] auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto ids = SeedIds{};
		auto rows_land =
		    tx.alloc(Grp.id)
		        .and_then([&](std::uint64_t minted) {
			        ids.grp = minted;
			        return tx.insert(Grp, GrpRow{.id = minted, .label = std::string{"algebra"}});
		        })
		        .and_then([&](bool) {
			        return tx.alloc(Course.id);
		        })
		        .and_then([&](std::uint64_t minted) {
			        ids.course = minted;
			        return tx.insert(Course, CourseRow{.id = minted, .grp = ids.grp, .title = std::string{"linear equations"}});
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

struct PendingReads {
	bool keyed_saw_pending;
	bool primary_saw_pending;
};

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r30") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r30 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r30 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), KeyedRead);
	if (!db.has_value()) {
		results.push_back(CaseResult{.name = "Db::ephemeral admits KeyedRead", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r30 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "the seed commits (one group, one course)",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	auto const by_group = db->get(Course, course_grp_key, {.grp = ids->grp});
	results.push_back(CaseResult{
	    .name = "db.get(Course, courseGrpKey, {grp}) answers the typed "
	            "point read",
	    .passed = by_group.has_value() && by_group->has_value() && (*by_group)->len() == 1 &&
	              cell_is_u64(**by_group, {.row = 0, .column = 0}, ids->course) &&
	              cell_is_text(**by_group, {.row = 0, .column = 2}, "linear equations"),
	});

	auto const by_id = db->get(Course, {.id = ids->course});
	results.push_back(CaseResult{
	    .name = "db.get(Course, {id}) answers the primary point read",
	    .passed =
	        by_id.has_value() && by_id->has_value() && (*by_id)->len() == 1 && cell_is_u64(**by_id, {.row = 0, .column = 1}, ids->grp),
	});

	auto const via_snap = db->read([&](bdb::Snapshot& snap) -> std::expected<bool, bdb::Error> {
		return snap.get(Course, course_grp_key, {.grp = ids->grp}).transform([&](std::optional<bdb::RowSet> rows) {
			return rows.has_value() && rows->len() == 1 && cell_is_u64(*rows, {.row = 0, .column = 0}, ids->course);
		});
	});
	results.push_back(CaseResult{
	    .name = "the read scope agrees with the standalone spelling",
	    .passed = via_snap.has_value() && *via_snap,
	});

	using MutateDecision = bdb::WriteDecision<PendingReads, std::monostate>;
	using MutateResult = std::expected<MutateDecision, bdb::Error>;
	auto mutated = db->write([&](bdb::WriteTx& tx) -> MutateResult {
		auto geometry = std::uint64_t{0};
		auto proofs = std::uint64_t{0};
		auto rows_land = tx.alloc(Grp.id)
		                     .and_then([&](std::uint64_t minted) {
			                     geometry = minted;
			                     return tx.insert(Grp, GrpRow{.id = minted, .label = std::string{"geometry"}});
		                     })
		                     .and_then([&](bool) {
			                     return tx.alloc(Course.id);
		                     })
		                     .and_then([&](std::uint64_t minted) {
			                     proofs = minted;
			                     return tx.insert(Course, CourseRow{.id = minted, .grp = geometry, .title = std::string{"proofs"}});
		                     });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		auto witnesses = PendingReads{};
		auto keyed = tx.get(Course, course_grp_key, {.grp = geometry});
		if (!keyed.has_value()) {
			return std::unexpected{std::move(keyed).error()};
		}
		witnesses.keyed_saw_pending = keyed->has_value() && (*keyed)->len() == 1 && cell_is_u64(**keyed, {.row = 0, .column = 0}, proofs);
		auto primary = tx.get(Course, {.id = proofs});
		if (!primary.has_value()) {
			return std::unexpected{std::move(primary).error()};
		}
		witnesses.primary_saw_pending =
		    primary->has_value() && (*primary)->len() == 1 && cell_is_text(**primary, {.row = 0, .column = 2}, "proofs");
		return bdb::commit(witnesses);
	});
	auto const committed = mutated.has_value() && std::holds_alternative<bdb::Committed<PendingReads>>(*mutated);
	results.push_back(CaseResult{
	    .name = "the keyed read-modify-write commits",
	    .passed = committed,
	});
	if (committed) {
		auto const& witnesses = std::get<bdb::Committed<PendingReads>>(*mutated).value;
		results.push_back(CaseResult{
		    .name = "the pending insert answers through the declared key",
		    .passed = witnesses.keyed_saw_pending,
		});
		results.push_back(CaseResult{
		    .name = "the primary form agrees pre-commit",
		    .passed = witnesses.primary_saw_pending,
		});
	}

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r30_keyed_read <fixtures-file>");
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
