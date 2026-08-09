import std;
import bumbledb;

inline constexpr auto Priority = bdb::closed<"Priority", "Low", "Normal", "Urgent">();

struct TicketRow {
	[[= bdb::fresh]] std::uint64_t id;

	bdb::ref_to<Priority.id> priority;
	std::int64_t opened_at;
};

inline constexpr auto Ticket = bdb::relation<"Ticket", TicketRow>;

inline constexpr auto Tickets = bdb::schema<"Tickets">(Priority, Ticket,

                                                       bdb::contained(bdb::on(Ticket.priority), bdb::on(Priority.id)));

inline constexpr auto Urgent = bdb::query(Tickets).rule([](auto r) consteval {
	auto vars = r.vars(Ticket);
	return r
	    .match(Ticket,
	           {
	               .id = vars.id,
	               .priority = Priority.Urgent,
	           })
	    .find({
	        .id = vars.id,
	    });
});

inline constexpr auto Actionable = bdb::query(Tickets).rule([](auto r) consteval {
	auto vars = r.vars(Ticket);
	return r
	    .match(Ticket,
	           {
	               .id = vars.id,
	               .priority = {Priority.Normal, Priority.Urgent},
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
	auto const dir = root / std::format("bumbledb-cookbook-r06-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

struct SeedIds {
	std::uint64_t low;
	std::uint64_t normal;
	std::uint64_t urgent;
};

[[nodiscard]] auto seed(bdb::Db& db) -> std::optional<SeedIds> {
	using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		auto low = tx.alloc(Ticket.id);
		if (!low.has_value()) {
			return std::unexpected{std::move(low).error()};
		}
		auto normal = tx.alloc(Ticket.id);
		if (!normal.has_value()) {
			return std::unexpected{std::move(normal).error()};
		}
		auto urgent = tx.alloc(Ticket.id);
		if (!urgent.has_value()) {
			return std::unexpected{std::move(urgent).error()};
		}
		auto rows_land = tx.insert(Ticket, TicketRow{.id = *low, .priority = Priority.Low, .opened_at = 10})
		                     .and_then([&](bool) {
			                     return tx.insert(Ticket, TicketRow{.id = *normal, .priority = Priority.Normal, .opened_at = 20});
		                     })
		                     .and_then([&](bool) {
			                     return tx.insert(Ticket, TicketRow{.id = *urgent, .priority = Priority.Urgent, .opened_at = 30});
		                     });
		if (!rows_land.has_value()) {
			return std::unexpected{std::move(rows_land).error()};
		}
		return bdb::commit(SeedIds{.low = *low, .normal = *normal, .urgent = *urgent});
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<SeedIds>>(*written).value;
}

template<auto Query>
[[nodiscard]] auto ticket_ids(bdb::Db& db, bdb::Prepared<Query>& prepared) -> std::optional<std::vector<std::uint64_t>> {
	auto result = db.read([&](bdb::Snapshot& snap) -> std::expected<std::vector<std::uint64_t>, bdb::Error> {
		return snap.execute(prepared, {}).transform([](bdb::Answers<Query> answers) {
			auto ids = std::vector<std::uint64_t>{};
			for (auto const& row : answers.rows()) {
				ids.push_back(row.id);
			}
			std::ranges::sort(ids);
			return ids;
		});
	});
	if (!result.has_value()) {
		return std::nullopt;
	}
	return *std::move(result);
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r06") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r06 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r06 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Tickets);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Tickets", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r06 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto const ids = seed(*db);
	results.push_back(CaseResult{
	    .name = "one ticket per priority commits (handle -> row id "
	            "marshal)",
	    .passed = ids.has_value(),
	});
	if (!ids.has_value()) {
		return;
	}

	auto urgent = db->prepare<Urgent>();
	auto actionable = db->prepare<Actionable>();
	results.push_back(CaseResult{
	    .name = "urgent / actionable prepare through the engine validator",
	    .passed = urgent.has_value() && actionable.has_value(),
	});
	if (!urgent.has_value() || !actionable.has_value()) {
		return;
	}

	auto const urgent_ids = ticket_ids(*db, *urgent);
	results.push_back(CaseResult{
	    .name = "urgent (handle literal) answers {urgent}",
	    .passed = urgent_ids.has_value() && *urgent_ids == std::vector{ids->urgent},
	});

	auto expected_actionable = std::vector{ids->normal, ids->urgent};
	std::ranges::sort(expected_actionable);
	auto const actionable_ids = ticket_ids(*db, *actionable);
	results.push_back(CaseResult{
	    .name = "actionable (membership array) answers {normal, urgent}",
	    .passed = actionable_ids.has_value() && *actionable_ids == expected_actionable,
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r06_tickets <fixtures-file>");
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
