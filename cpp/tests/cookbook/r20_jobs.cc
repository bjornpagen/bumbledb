import std;
import bumbledb;

inline constexpr auto State = bdb::closed<"State", "Queued", "Running", "Done">();

struct JobRow {
	[[= bdb::fresh]] std::uint64_t id;

	bdb::ref_to<State.id> state;
	std::string payload;
};

struct LeaseRow {
	std::uint64_t job;
	std::uint64_t worker;
	std::int64_t until;
};

inline constexpr auto Job = bdb::relation<"Job", JobRow>;
inline constexpr auto Lease = bdb::relation<"Lease", LeaseRow>;

inline constexpr auto Jobs =
    bdb::schema<"Jobs">(State, Job, Lease,

                        bdb::contained(bdb::on(Job.state), bdb::on(State.id)), bdb::key(Lease.job),

                        bdb::mirrors(bdb::on(Lease.job), bdb::on(bdb::where(Job, {.state = State.Running}), Job.id)));

inline constexpr auto StillQueued = bdb::query(Jobs).rule([](auto r) consteval {
	auto vars = r.vars(Job);
	return r
	    .match(Job,
	           {
	               .id = vars.id,
	               .state = State.Queued,
	               .payload = vars.payload,
	           })
	    .find({
	        .id = vars.id,
	        .payload = vars.payload,
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
	auto const dir = root / std::format("bumbledb-cookbook-r20-{:08x}{:08x}", device(), device());
	std::filesystem::remove_all(dir, code);
	code.clear();
	std::filesystem::create_directories(dir, code);
	if (code) {
		return std::nullopt;
	}
	return dir;
}

auto run_cases(std::string_view fixtures_path, std::vector<CaseResult>& results) -> void {
	auto const fixtures = slurp(fixtures_path);
	auto const golden = fixtures.has_value() ? golden_of(*fixtures, "r20") : std::nullopt;
	if (!golden.has_value()) {
		results.push_back(CaseResult{.name = "fixtures file carries an r20 line", .passed = false});
		return;
	}

	auto const dir = make_store_dir();
	if (!dir.has_value()) {
		results.push_back(CaseResult{.name = "r20 store directory", .passed = false});
		return;
	}
	auto db = bdb::Db::ephemeral(dir->native(), Jobs);
	if (!db.has_value()) {
		std::println("  Db::ephemeral: {}", db.error().message());
		results.push_back(CaseResult{.name = "Db::ephemeral admits Jobs", .passed = false});
		return;
	}

	auto const fingerprint = db->fingerprint();
	results.push_back(CaseResult{
	    .name = "r20 fingerprint matches the pinned golden",
	    .passed = fingerprint.has_value() && *fingerprint == *golden,
	});

	auto prepared = db->prepare<StillQueued>();
	results.push_back(CaseResult{
	    .name = "stillQueued (the handle-literal premise) prepares "
	            "through the engine validator",
	    .passed = prepared.has_value(),
	});
	if (!prepared.has_value()) {
		return;
	}

	auto outcome = db->write_witnessed([&](bdb::Snapshot& snap,
	                                       bdb::WriteTx& tx) -> std::expected<bdb::WriteDecision<std::monostate, std::string>, bdb::Error> {
		auto queued = snap.execute(*prepared, {});
		if (!queued.has_value()) {
			return std::unexpected{std::move(queued).error()};
		}
		if (queued->size() == 0) {
			return bdb::abandon(std::string{"nothing queued"});
		}
		for (auto const& row : queued->rows()) {
			auto moved = tx.remove(Job, JobRow{.id = row.id, .state = State.Queued, .payload = std::string{row.payload}})
			                 .and_then([&](bool) {
				                 return tx.insert(Job, JobRow{.id = row.id, .state = State.Running, .payload = std::string{row.payload}});
			                 })
			                 .and_then([&](bool) {
				                 return tx.insert(Lease, LeaseRow{.job = row.id, .worker = 7, .until = 60});
			                 });
			if (!moved.has_value()) {
				return std::unexpected{std::move(moved).error()};
			}
		}
		return bdb::commit();
	});

	auto abandoned = outcome.has_value() && std::holds_alternative<bdb::Abandoned<std::string>>(*outcome);
	results.push_back(CaseResult{
	    .name = "the empty store has nothing queued — the loop abandons",
	    .passed = abandoned,
	});
	results.push_back(CaseResult{
	    .name = "the abandonment carries its own payload "
	            "(\"nothing queued\")",
	    .passed = abandoned && std::get<bdb::Abandoned<std::string>>(*outcome).value == "nothing queued",
	});

	auto code = std::error_code{};
	std::filesystem::remove_all(*dir, code);
}

}

auto main(int argc, char** argv) -> int {
	auto const arguments = std::span{argv, static_cast<std::size_t>(argc)};
	if (arguments.size() < 2) {
		std::println("FAIL: usage: r20_jobs <fixtures-file>");
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
