import std;
import bumbledb;

struct SeverityPayload {
	bool pages;
};

inline constexpr auto Severity = bdb::closed<"Severity", SeverityPayload>(
    bdb::member<"Info">(SeverityPayload{.pages = false}), bdb::member<"Warning">(SeverityPayload{.pages = false}),
    bdb::member<"Critical">(SeverityPayload{.pages = true}), bdb::member<"Fatal">(SeverityPayload{.pages = true}));

struct IncidentRow {
	[[= bdb::fresh]] std::uint64_t id;

	bdb::ref_to<Severity.id> severity;
};

struct EscalationRow {
	std::uint64_t incident;
	bdb::ref_to<Severity.id> severity;
	std::int64_t at;
};

inline constexpr auto Incident = bdb::relation<"Incident", IncidentRow>;
inline constexpr auto Escalation = bdb::relation<"Escalation", EscalationRow>;

inline constexpr auto Oncall = bdb::schema<"Oncall">(
    Severity, Incident, Escalation,

    bdb::contained(bdb::on(Incident.severity), bdb::on(Severity.id)), bdb::contained(bdb::on(Escalation.incident), bdb::on(Incident.id)),

    bdb::contained(bdb::on(Escalation.severity), bdb::on(bdb::where(Severity, {.pages = true}), Severity.id)));

inline constexpr auto Paged = bdb::query(Oncall).rule([](auto r) consteval {
	auto vars = r.vars(Escalation);
	return r
	    .match(Escalation,
	           {
	               .incident = vars.incident,
	               .severity = vars.severity,
	           })
	    .match(Severity,
	           {
	               .id = vars.severity,
	               .pages = true,
	           })
	    .find({
	        .incident = vars.incident,
	    });
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
	auto const dir = root / std::format("bumbledb-cookbook-r08-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t quiet;
	std::uint64_t paged;
};

[[nodiscard]] auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto quiet = tx.alloc(Incident.id);
		if (!quiet.has_value()) {
			return std::unexpected{std::move(quiet).error()};
		}
		auto paged = tx.alloc(Incident.id);
		if (!paged.has_value()) {
			return std::unexpected{std::move(paged).error()};
		}
		auto rows_land = tx.insert(Incident, IncidentRow{.id = *quiet, .severity = Severity.Info})
		                     .and_then([&](bool) {
			                     return tx.insert(Incident, IncidentRow{.id = *paged, .severity = Severity.Critical});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Escalation, EscalationRow{.incident = *paged, .severity = Severity.Critical, .at = 100});
		                     });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(SeedIds{.quiet = *quiet, .paged = *paged});
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<SeedIds>>(*written).value;
}

[[nodiscard]] auto info_escalation_rejected(bdb::Db& db, std::uint64_t incident) -> bool {
	using Decision = bdb::WriteDecision<std::monostate, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto landed = tx.insert(Escalation, EscalationRow{.incident = incident, .severity = Severity.Info, .at = 5});
		if (!landed.has_value()) {
			return std::unexpected{std::move(landed).error()};
		}
		return bdb::commit();
	});
	return !written.has_value() && written.error().kind() == bdb::ErrorKind::CommitRejected;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r08") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r08 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r08 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Oncall);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Oncall", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r08 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "incidents + a Critical escalation commit (ψ admits the "
	            "paging member set)",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	results.push_back(CaseResult{
	    .name = "an escalation at Info is commit-rejected (the typed "
	            "CommitRejected error)",
	    .passed = info_escalation_rejected(*db, ids->quiet),
	});

	auto paged = db->prepare<Paged>();
	results.push_back(CaseResult{
	    .name = "paged prepares through the engine validator",
	    .passed = paged.has_value(),
	});
	if (!paged.has_value()) {
		return;
	}

	auto answered = db->read([&](bdb::Snapshot& snap) -> std::expected<std::vector<std::uint64_t>, bdb::Error> {
		return snap.execute(*paged, {}).transform([](bdb::Answers<Paged> answers) {
			auto incidents = std::vector<std::uint64_t>{};
			for (auto const& row : answers.rows()) {
				incidents.push_back(row.incident);
			}
			std::ranges::sort(incidents);
			return incidents;
		});
	});
	results.push_back(CaseResult{
	    .name = "paged answers {the Critical incident}",
	    .passed = answered.has_value() && *answered == std::vector{ids->paged},
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r08_oncall <fixtures-file>");
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
