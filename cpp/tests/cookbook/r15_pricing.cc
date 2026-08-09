// Cookbook recipe 15 — Effective-dated configuration (ts/COOKBOOK.md §15):
// versioned rules as a DISJOINT COVER. No overlapping versions (the
// pointwise key), no gaps in the policy's source lifetime (the one-way
// coverage containment — version overhang stays legal), and "in force on
// date t" is one membership probe.
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r15 <64-hex>"); inForce (point_in vs a param) and successions (the
// VAR-VAR `allen(a, meets, b)` self-join) prepare AND answer the recipe's
// own semantics; an uncovered policy lifetime and an overlapping version
// are both commit-rejected (the disjoint-cover judgment, engine-side).
//
// The successions self-join's second interval variable: the TS rule mints
// a second `v(Version)`; the C++ dialect's variable identity is the mint
// coordinate, so the second variable of the SAME law class is minted at
// the class's other coordinate — `Policy.live`, unified with
// `Version.valid` by the coverage containment (TODO_CPP §10: the paired
// statement IS the semantic fact that unifies the coordinates).
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

struct PolicyRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    bdb::interval<std::int64_t> live;
};

struct VersionRow {
    std::uint64_t policy;
    std::int64_t rate_bps;
    bdb::interval<std::int64_t> valid;
};

inline constexpr auto Policy = bdb::relation<"Policy", PolicyRow>;
inline constexpr auto Version = bdb::relation<"Version", VersionRow>;

inline constexpr auto Pricing = bdb::schema<"Pricing">(
    Policy,
    Version,

    bdb::contained(
        bdb::on(Version.policy),
        bdb::on(Policy.id)
    ),

    // No overlapping versions: at any instant, at most one rate is the
    // law.
    bdb::key(
        Version.policy,
        Version.valid
    ),

    // No gaps in the policy lifetime: every source point is covered by
    // versions. Together with the key above this is a disjoint cover, not
    // an exact partition: Version intervals may overhang (recipe 16).
    bdb::contained(
        bdb::on(Policy.id, Policy.live),
        bdb::on(Version.policy, Version.valid)
    )
);

// in force on date t — one membership probe.
inline constexpr auto InForce =
    bdb::query(Pricing).rule([](auto r) consteval {
        auto vars = r.vars(Version);
        return r
            .match(Version,
                {
                    .policy = bdb::param<"p">(),
                    .rate_bps = vars.rate_bps,
                    .valid = vars.valid,
                })
            .where(bdb::point_in(bdb::param<"t">(), vars.valid))
            .find({
                .rate_bps = vars.rate_bps,
            });
    });

// clean successions (half-open makes MEETS exact, no ±1 fudge): the same
// relation matched twice, the second `valid` variable minted at the law
// class's other coordinate (module comment).
inline constexpr auto Successions =
    bdb::query(Pricing).rule([](auto r) consteval {
        auto version = r.vars(Version);
        auto policy = r.vars(Policy);
        return r
            .match(Version,
                {
                    .policy = version.policy,
                    .valid = version.valid,
                })
            .match(Version,
                {
                    .policy = version.policy,
                    .valid = policy.live,
                })
            .where(bdb::allen_in(version.valid, bdb::allen::meets,
                policy.live))
            .find({}, bdb::as<"a">(version.valid),
                bdb::as<"b">(policy.live));
    });

namespace {

struct CaseResult {
    std::string name;
    bool passed;
};

/// The golden of one recipe: the fixtures file is one `rNN <64-hex>` line
/// per recipe (ts/test/cookbook.test.ts reads the same file).
auto golden_of(std::string_view fixtures, std::string_view recipe)
    -> std::optional<std::string> {
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
    auto stream = std::ifstream{std::string{path},
        std::ios::binary | std::ios::ate};
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
    auto const dir = root
        / std::format("bumbledb-cookbook-r15-{:08x}{:08x}", device(),
            device());
    std::filesystem::remove_all(dir, code);
    code.clear();
    std::filesystem::create_directories(dir, code);
    if (code) {
        return std::nullopt;
    }
    return dir;
}

/// One policy live [0,20), covered by three meeting versions — the third
/// overhangs the lifetime (legal: coverage is one-way).
///
///   [0,10) @ 100    [10,20) @ 200    [20,25) @ 300
auto seed(bdb::Db& db) -> std::optional<std::uint64_t> {
    using Decision = bdb::WriteDecision<std::uint64_t, std::monostate>;
    using Result = std::expected<Decision, bdb::Error>;
    auto written = db.write([&](bdb::WriteTx& tx) -> Result {
        auto policy = tx.alloc(Policy.id);
        if (!policy.has_value()) {
            return std::unexpected{std::move(policy).error()};
        }
        auto rows_land =
            tx.insert(Policy,
                  PolicyRow{.id = *policy,
                      .live = bdb::interval<std::int64_t>::literal(0, 20)})
                .and_then([&](bool) {
                    return tx.insert(Version,
                        VersionRow{.policy = *policy,
                            .rate_bps = 100,
                            .valid = bdb::interval<std::int64_t>::literal(
                                0, 10)});
                })
                .and_then([&](bool) {
                    return tx.insert(Version,
                        VersionRow{.policy = *policy,
                            .rate_bps = 200,
                            .valid = bdb::interval<std::int64_t>::literal(
                                10, 20)});
                })
                .and_then([&](bool) {
                    return tx.insert(Version,
                        VersionRow{.policy = *policy,
                            .rate_bps = 300,
                            .valid = bdb::interval<std::int64_t>::literal(
                                20, 25)});
                });
        if (!rows_land.has_value()) {
            return std::unexpected{std::move(rows_land).error()};
        }
        return bdb::commit(*policy);
    });
    if (!written.has_value()
        || !std::holds_alternative<bdb::Committed<std::uint64_t>>(
            *written)) {
        return std::nullopt;
    }
    return std::get<bdb::Committed<std::uint64_t>>(*written).value;
}

/// inForce(p, t), rates sorted (answers are sets; the host sorts).
auto rates_in_force(bdb::Db& db, bdb::Prepared<InForce>& prepared,
    std::uint64_t policy, std::int64_t at)
    -> std::optional<std::vector<std::int64_t>> {
    auto result = db.execute(prepared, {.p = policy, .t = at})
        .transform([](bdb::Answers<InForce> answers) {
            auto rates = std::vector<std::int64_t>{};
            for (auto const& row : answers.rows()) {
                rates.push_back(row.rate_bps);
            }
            std::ranges::sort(rates);
            return rates;
        });
    if (!result.has_value()) {
        return std::nullopt;
    }
    return *std::move(result);
}

auto run_cases(std::string_view fixtures_path,
    std::vector<CaseResult>& results) -> void {
    auto const fixtures = slurp(fixtures_path);
    auto const golden =
        fixtures.has_value() ? golden_of(*fixtures, "r15") : std::nullopt;
    if (!golden.has_value()) {
        results.push_back(CaseResult{
            .name = "fixtures file carries an r15 line", .passed = false});
        return;
    }

    auto const dir = make_store_dir();
    if (!dir.has_value()) {
        results.push_back(CaseResult{
            .name = "r15 store directory", .passed = false});
        return;
    }
    auto db = bdb::Db::ephemeral(dir->native(), Pricing);
    if (!db.has_value()) {
        std::println("  Db::ephemeral: {}", db.error().message());
        results.push_back(CaseResult{
            .name = "Db::ephemeral admits Pricing", .passed = false});
        return;
    }

    auto const fingerprint = db->fingerprint();
    results.push_back(CaseResult{
        .name = "r15 fingerprint matches the pinned golden",
        .passed = fingerprint.has_value() && *fingerprint == *golden,
    });

    auto const policy = seed(*db);
    results.push_back(CaseResult{
        .name = "a covered policy lifetime with an overhanging tail "
                "version commits (the disjoint cover admits overhang)",
        .passed = policy.has_value(),
    });
    if (!policy.has_value()) {
        return;
    }

    auto in_force = db->prepare<InForce>();
    auto successions = db->prepare<Successions>();
    results.push_back(CaseResult{
        .name = "inForce / successions prepare through the engine "
                "validator",
        .passed = in_force.has_value() && successions.has_value(),
    });
    if (!in_force.has_value() || !successions.has_value()) {
        return;
    }

    // The membership probe: exactly one rate is the law at any instant.
    auto const at_5 = rates_in_force(*db, *in_force, *policy, 5);
    results.push_back(CaseResult{
        .name = "inForce(t=5) answers {100}",
        .passed = at_5.has_value()
            && *at_5 == std::vector<std::int64_t>{100},
    });
    auto const at_15 = rates_in_force(*db, *in_force, *policy, 15);
    results.push_back(CaseResult{
        .name = "inForce(t=15) answers {200}",
        .passed = at_15.has_value()
            && *at_15 == std::vector<std::int64_t>{200},
    });
    auto const at_22 = rates_in_force(*db, *in_force, *policy, 22);
    results.push_back(CaseResult{
        .name = "inForce(t=22) answers {300} (the overhang is in force)",
        .passed = at_22.has_value()
            && *at_22 == std::vector<std::int64_t>{300},
    });
    auto const at_30 = rates_in_force(*db, *in_force, *policy, 30);
    results.push_back(CaseResult{
        .name = "inForce(t=30) answers the empty set",
        .passed = at_30.has_value() && at_30->empty(),
    });

    // Successions: exactly the two meeting pairs, boundary-exact.
    auto pairs = db->execute(*successions, {})
        .transform([](bdb::Answers<Successions> answers) {
            auto out = std::vector<std::pair<bdb::interval<std::int64_t>,
                bdb::interval<std::int64_t>>>{};
            for (auto const& row : answers.rows()) {
                out.emplace_back(row.a, row.b);
            }
            std::ranges::sort(out, {},
                [](auto const& row) { return row.first.lo(); });
            return out;
        });
    auto const first = bdb::interval<std::int64_t>::literal(0, 10);
    auto const second = bdb::interval<std::int64_t>::literal(10, 20);
    auto const third = bdb::interval<std::int64_t>::literal(20, 25);
    results.push_back(CaseResult{
        .name = "successions answers the two meeting pairs "
                "([0,10)->[10,20), [10,20)->[20,25))",
        .passed = pairs.has_value() && pairs->size() == 2
            && (*pairs)[0] == std::pair{first, second}
            && (*pairs)[1] == std::pair{second, third},
    });

    // A policy lifetime with no covering versions violates the coverage
    // containment.
    auto uncovered = db->write([&](bdb::WriteTx& tx)
            -> std::expected<
                bdb::WriteDecision<std::monostate, std::monostate>,
                bdb::Error> {
        auto landed = tx.alloc(Policy.id).and_then(
            [&](std::uint64_t minted) {
                return tx.insert(Policy,
                    PolicyRow{.id = minted,
                        .live = bdb::interval<std::int64_t>::literal(
                            100, 110)});
            });
        if (!landed.has_value()) {
            return std::unexpected{std::move(landed).error()};
        }
        return bdb::commit();
    });
    results.push_back(CaseResult{
        .name = "an uncovered policy lifetime is commit-rejected (no "
                "gaps)",
        .passed = !uncovered.has_value()
            && uncovered.error().kind() == bdb::ErrorKind::CommitRejected
            && !uncovered.error().violations().empty(),
    });

    // Two versions sharing an instant violate the pointwise key.
    auto overlapping = db->write([&](bdb::WriteTx& tx)
            -> std::expected<
                bdb::WriteDecision<std::monostate, std::monostate>,
                bdb::Error> {
        auto landed = tx.insert(Version,
            VersionRow{.policy = *policy,
                .rate_bps = 999,
                .valid = bdb::interval<std::int64_t>::literal(22, 27)});
        if (!landed.has_value()) {
            return std::unexpected{std::move(landed).error()};
        }
        return bdb::commit();
    });
    results.push_back(CaseResult{
        .name = "an overlapping version is commit-rejected (no shared "
                "instant)",
        .passed = !overlapping.has_value()
            && overlapping.error().kind()
                == bdb::ErrorKind::CommitRejected
            && !overlapping.error().violations().empty(),
    });

    auto code = std::error_code{};
    std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
    auto const arguments =
        std::span{argv, static_cast<std::size_t>(argc)};
    if (arguments.size() < 2) {
        std::println("FAIL: usage: r15_pricing <fixtures-file>");
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
