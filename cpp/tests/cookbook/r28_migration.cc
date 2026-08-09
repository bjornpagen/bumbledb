import std;
import bumbledb;

struct EmployeeRow {
	[[= bdb::fresh]] std::uint64_t id;

	std::string name;
};

struct SalaryV1Row {
	std::uint64_t employee;
	std::int64_t amount;
};

struct SalaryRow {
	std::uint64_t employee;
	std::int64_t amount;
	bdb::interval<std::int64_t> applies;
};

inline constexpr auto Employee = bdb::relation<"Employee", EmployeeRow>;
inline constexpr auto SalaryV1 = bdb::relation<"Salary", SalaryV1Row>;
inline constexpr auto Salary = bdb::relation<"Salary", SalaryRow>;

inline constexpr auto PayrollV1 = bdb::schema<"PayrollV1">(Employee, SalaryV1,

                                                           bdb::contained(bdb::on(SalaryV1.employee), bdb::on(Employee.id)));

inline constexpr auto Payroll = bdb::schema<"Payroll">(Employee, Salary,

                                                       bdb::contained(bdb::on(Salary.employee), bdb::on(Employee.id)),

                                                       bdb::key(Salary.employee, Salary.applies));

inline constexpr auto InForceAt = bdb::query(Payroll).rule([](auto r) consteval {
	auto employee = r.vars(Employee);
	auto salary = r.vars(Salary);
	return r
	    .match(Employee,
	           {
	               .id = employee.id,
	               .name = employee.name,
	           })
	    .match(Salary,
	           {
	               .employee = employee.id,
	               .amount = salary.amount,
	               .applies = salary.applies,
	           })
	    .where(bdb::point_in(bdb::param<"at">(), salary.applies))
	    .find({
	        .name = employee.name,
	        .amount = salary.amount,
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

[[nodiscard]] auto make_store_dir(std::string_view label) -> std::optional<std::filesystem::path> {
	auto code = std::error_code{};
	auto const root = std::filesystem::temp_directory_path(code);
	if (code) {
		return std::nullopt;
	}
	auto device = std::random_device{};
	auto const dir = root / std::format("bumbledb-cookbook-r28-{}-{:08x}{:08x}", label, device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

[[nodiscard]] auto seed_v2(bdb::Db& db) -> std::optional<std::uint64_t> {
	using Decision = bdb::WriteDecision<std::uint64_t, std::monostate>;
	using Result = std::expected<Decision, bdb::Error>;
	auto written = db.write([&](bdb::WriteTx& tx) -> Result {
		return tx.alloc(Employee.id).and_then([&](std::uint64_t ada) -> Result {
			return tx.insert(Employee, EmployeeRow{.id = ada, .name = std::string{"ada"}})
			    .and_then([&](bool) {
				    return tx.insert(Salary,
				                     SalaryRow{.employee = ada, .amount = 100, .applies = bdb::interval<std::int64_t>::literal(0, 100)});
			    })
			    .and_then([&](bool) {
				    return tx.insert(Salary,
				                     SalaryRow{.employee = ada, .amount = 120, .applies = bdb::interval<std::int64_t>::literal(100, 200)});
			    })
			    .transform([ada](bool) -> Decision {
				    return bdb::commit(ada);
			    });
		});
	});
	if (!written.has_value() || !std::holds_alternative<bdb::Committed<std::uint64_t>>(*written)) {
		return std::nullopt;
	}
	return std::get<bdb::Committed<std::uint64_t>>(*written).value;
}

[[nodiscard]] auto in_force(bdb::Db& db, bdb::Prepared<InForceAt>& prepared, std::int64_t at)
    -> std::optional<std::vector<std::pair<std::string, std::int64_t>>> {
	auto result = db.read([&](bdb::Snapshot& snap) -> std::expected<std::vector<std::pair<std::string, std::int64_t>>, bdb::Error> {
		return snap.execute(prepared, {.at = at}).transform([](bdb::Answers<InForceAt> answers) {
			auto rows = std::vector<std::pair<std::string, std::int64_t>>{};
			for (auto const& row : answers.rows()) {
				rows.emplace_back(std::string{row.name}, row.amount);
			}
			std::ranges::sort(rows);
			return rows;
		});
	});
	if (!result.has_value()) {
		return std::nullopt;
	}
	return *std::move(result);
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r28") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r28 line", .passed = false});
		return;
	}

	auto const v1_dir = make_store_dir("v1");
	auto const v2_dir = make_store_dir("v2");
	if (!v1_dir.has_value() || !v2_dir.has_value()) {
		results.push_back(CaseResult{.name = "r28 store directories", .passed = false});
		return;
	}

	auto v1 = bdb::Db::ephemeral(v1_dir->native(), PayrollV1);
	auto v2 = bdb::Db::ephemeral(v2_dir->native(), Payroll);
	results.push_back(CaseResult{
	    .name = "the engine admits both the v1 and the v2 theory",
	    .passed = v1.has_value() && v2.has_value(),
	});
	if (!v1.has_value() || !v2.has_value()) {
		return;
	}

	auto const v1_fingerprint = v1->fingerprint();
	auto const v2_fingerprint = v2->fingerprint();
	if (!v1_fingerprint.has_value() || !v2_fingerprint.has_value()) {
		results.push_back(CaseResult{.name = "both fingerprints read back", .passed = false});
		return;
	}

	results.push_back(CaseResult{
	    .name = "the v2 fingerprint matches the pinned r28 golden",
	    .passed = *v2_fingerprint == *golden,
	});

	results.push_back(CaseResult{
	    .name = "the v1 and v2 theories carry two distinct fingerprints",
	    .passed = *v1_fingerprint != *v2_fingerprint,
	});

	auto in_force_at = v2->prepare<InForceAt>();
	results.push_back(CaseResult{
	    .name = "inForceAt prepares through the engine validator",
	    .passed = in_force_at.has_value(),
	});
	if (!in_force_at.has_value()) {
		return;
	}

	auto const ada = seed_v2(*v2);
	results.push_back(CaseResult{
	    .name = "the transformed load commits under the v2 theory",
	    .passed = ada.has_value(),
	});
	if (!ada.has_value()) {
		return;
	}

	auto const at_50 = in_force(*v2, *in_force_at, 50);
	results.push_back(CaseResult{
	    .name = "inForceAt(50) answers {(ada, 100)}",
	    .passed = at_50.has_value() && at_50->size() == 1 && (*at_50)[0] == std::pair{std::string{"ada"}, std::int64_t{100}},
	});
	auto const at_150 = in_force(*v2, *in_force_at, 150);
	results.push_back(CaseResult{
	    .name = "inForceAt(150) answers {(ada, 120)}",
	    .passed = at_150.has_value() && at_150->size() == 1 && (*at_150)[0] == std::pair{std::string{"ada"}, std::int64_t{120}},
	});
	auto const at_500 = in_force(*v2, *in_force_at, 500);
	results.push_back(CaseResult{
	    .name = "inForceAt(500) answers the empty set",
	    .passed = at_500.has_value() && at_500->empty(),
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*v1_dir, code);
	code.clear();
	std::filesystem::remove_all(*v2_dir, code);
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r28_migration <fixtures-file>");
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
