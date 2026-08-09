// Cookbook recipe 16 — Disjoint covers (ts/COOKBOOK.md §16): pay periods,
// shifts, estimated-tax quarters. A pointwise key plus one-way coverage is
// a DISJOINT COVER — no overlaps among pay periods and no holes in the
// fiscal year's source span; pay periods may extend beyond that span.
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r16 <64-hex>"); holding (the period holding date t — point_in vs a
// param) prepares AND answers the recipe's own semantics; an overlapping
// period, a duplicated sequence number, and a holed fiscal year are all
// commit-rejected.
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

struct FiscalYearRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    bdb::interval<std::int64_t> span;
};

struct PayPeriodRow {
    std::uint64_t year;
    std::uint64_t seq;
    bdb::interval<std::int64_t> span;
};

inline constexpr auto FiscalYear = bdb::relation<"FiscalYear", FiscalYearRow>;
inline constexpr auto PayPeriod = bdb::relation<"PayPeriod", PayPeriodRow>;

inline constexpr auto Payroll = bdb::schema<"Payroll">(
    FiscalYear,
    PayPeriod,

    bdb::contained(
        bdb::on(PayPeriod.year),
        bdb::on(FiscalYear.id)
    ),

    // Sequence numbers stay unique.
    bdb::key(
        PayPeriod.year,
        PayPeriod.seq
    ),

    // Disjoint: no shared instant.
    bdb::key(
        PayPeriod.year,
        PayPeriod.span
    ),

    // Covering: no holes in the fiscal year's span; overhang is legal.
    bdb::contained(
        bdb::on(FiscalYear.id, FiscalYear.span),
        bdb::on(PayPeriod.year, PayPeriod.span)
    )
);

// the period holding date t.
inline constexpr auto Holding =
    bdb::query(Payroll).rule([](auto r) consteval {
        auto vars = r.vars(PayPeriod);
        return r
            .match(PayPeriod,
                {
                    .year = bdb::param<"y">(),
                    .seq = vars.seq,
                    .span = vars.span,
                })
            .where(bdb::point_in(bdb::param<"t">(), vars.span))
            .find({
                .seq = vars.seq,
            });
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
        / std::format("bumbledb-cookbook-r16-{:08x}{:08x}", device(),
            device());
    std::filesystem::remove_all(dir, code);
    code.clear();
    std::filesystem::create_directories(dir, code);
    if (code) {
        return std::nullopt;
    }
    return dir;
}

/// One fiscal year spanning [0,30), covered by three meeting periods —
/// the last extends beyond the year's span (overhang is legal).
///
///   seq 1 [0,10)    seq 2 [10,20)    seq 3 [20,35)
auto seed(bdb::Db& db) -> std::optional<std::uint64_t> {
    using Decision = bdb::WriteDecision<std::uint64_t, std::monostate>;
    using Result = std::expected<Decision, bdb::Error>;
    auto written = db.write([&](bdb::WriteTx& tx) -> Result {
        auto year = tx.alloc(FiscalYear.id);
        if (!year.has_value()) {
            return std::unexpected{std::move(year).error()};
        }
        auto rows_land =
            tx.insert(FiscalYear,
                  FiscalYearRow{.id = *year,
                      .span = bdb::interval<std::int64_t>::literal(0, 30)})
                .and_then([&](bool) {
                    return tx.insert(PayPeriod,
                        PayPeriodRow{.year = *year,
                            .seq = 1,
                            .span = bdb::interval<std::int64_t>::literal(
                                0, 10)});
                })
                .and_then([&](bool) {
                    return tx.insert(PayPeriod,
                        PayPeriodRow{.year = *year,
                            .seq = 2,
                            .span = bdb::interval<std::int64_t>::literal(
                                10, 20)});
                })
                .and_then([&](bool) {
                    return tx.insert(PayPeriod,
                        PayPeriodRow{.year = *year,
                            .seq = 3,
                            .span = bdb::interval<std::int64_t>::literal(
                                20, 35)});
                });
        if (!rows_land.has_value()) {
            return std::unexpected{std::move(rows_land).error()};
        }
        return bdb::commit(*year);
    });
    if (!written.has_value()
        || !std::holds_alternative<bdb::Committed<std::uint64_t>>(
            *written)) {
        return std::nullopt;
    }
    return std::get<bdb::Committed<std::uint64_t>>(*written).value;
}

/// holding(y, t), sequence numbers sorted (answers are sets; the host
/// sorts).
auto holding_seqs(bdb::Db& db, bdb::Prepared<Holding>& prepared,
    std::uint64_t year, std::int64_t at)
    -> std::optional<std::vector<std::uint64_t>> {
    auto result = db.execute(prepared, {.y = year, .t = at})
        .transform([](bdb::Answers<Holding> answers) {
            auto seqs = std::vector<std::uint64_t>{};
            for (auto const& row : answers.rows()) {
                seqs.push_back(row.seq);
            }
            std::ranges::sort(seqs);
            return seqs;
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
        fixtures.has_value() ? golden_of(*fixtures, "r16") : std::nullopt;
    if (!golden.has_value()) {
        results.push_back(CaseResult{
            .name = "fixtures file carries an r16 line", .passed = false});
        return;
    }

    auto const dir = make_store_dir();
    if (!dir.has_value()) {
        results.push_back(CaseResult{
            .name = "r16 store directory", .passed = false});
        return;
    }
    auto db = bdb::Db::ephemeral(dir->native(), Payroll);
    if (!db.has_value()) {
        std::println("  Db::ephemeral: {}", db.error().message());
        results.push_back(CaseResult{
            .name = "Db::ephemeral admits Payroll", .passed = false});
        return;
    }

    auto const fingerprint = db->fingerprint();
    results.push_back(CaseResult{
        .name = "r16 fingerprint matches the pinned golden",
        .passed = fingerprint.has_value() && *fingerprint == *golden,
    });

    auto const year = seed(*db);
    results.push_back(CaseResult{
        .name = "a covered fiscal year with an overhanging final period "
                "commits (the disjoint cover admits overhang)",
        .passed = year.has_value(),
    });
    if (!year.has_value()) {
        return;
    }

    auto holding = db->prepare<Holding>();
    results.push_back(CaseResult{
        .name = "holding prepares through the engine validator",
        .passed = holding.has_value(),
    });
    if (!holding.has_value()) {
        return;
    }

    // The membership probe: exactly one period holds any covered instant.
    auto const at_5 = holding_seqs(*db, *holding, *year, 5);
    results.push_back(CaseResult{
        .name = "holding(t=5) answers {1}",
        .passed = at_5.has_value()
            && *at_5 == std::vector<std::uint64_t>{1},
    });
    auto const at_25 = holding_seqs(*db, *holding, *year, 25);
    results.push_back(CaseResult{
        .name = "holding(t=25) answers {3}",
        .passed = at_25.has_value()
            && *at_25 == std::vector<std::uint64_t>{3},
    });
    auto const at_32 = holding_seqs(*db, *holding, *year, 32);
    results.push_back(CaseResult{
        .name = "holding(t=32) answers {3} (the overhang still holds "
                "its instants)",
        .passed = at_32.has_value()
            && *at_32 == std::vector<std::uint64_t>{3},
    });
    auto const at_40 = holding_seqs(*db, *holding, *year, 40);
    results.push_back(CaseResult{
        .name = "holding(t=40) answers the empty set",
        .passed = at_40.has_value() && at_40->empty(),
    });

    // Two periods sharing an instant violate the pointwise key.
    auto overlapping = db->write([&](bdb::WriteTx& tx)
            -> std::expected<
                bdb::WriteDecision<std::monostate, std::monostate>,
                bdb::Error> {
        auto landed = tx.insert(PayPeriod,
            PayPeriodRow{.year = *year,
                .seq = 4,
                .span = bdb::interval<std::int64_t>::literal(15, 25)});
        if (!landed.has_value()) {
            return std::unexpected{std::move(landed).error()};
        }
        return bdb::commit();
    });
    results.push_back(CaseResult{
        .name = "an overlapping period is commit-rejected (no shared "
                "instant)",
        .passed = !overlapping.has_value()
            && overlapping.error().kind()
                == bdb::ErrorKind::CommitRejected
            && !overlapping.error().violations().empty(),
    });

    // A reused sequence number violates the (year, seq) key even on a
    // disjoint span.
    auto duplicated = db->write([&](bdb::WriteTx& tx)
            -> std::expected<
                bdb::WriteDecision<std::monostate, std::monostate>,
                bdb::Error> {
        auto landed = tx.insert(PayPeriod,
            PayPeriodRow{.year = *year,
                .seq = 2,
                .span = bdb::interval<std::int64_t>::literal(40, 50)});
        if (!landed.has_value()) {
            return std::unexpected{std::move(landed).error()};
        }
        return bdb::commit();
    });
    results.push_back(CaseResult{
        .name = "a duplicated sequence number is commit-rejected",
        .passed = !duplicated.has_value()
            && duplicated.error().kind()
                == bdb::ErrorKind::CommitRejected
            && !duplicated.error().violations().empty(),
    });

    // A fiscal year whose span has a hole violates the coverage
    // containment.
    auto holed = db->write([&](bdb::WriteTx& tx)
            -> std::expected<
                bdb::WriteDecision<std::monostate, std::monostate>,
                bdb::Error> {
        auto landed = tx.alloc(FiscalYear.id)
            .and_then([&](std::uint64_t minted) {
                return tx.insert(FiscalYear,
                        FiscalYearRow{.id = minted,
                            .span = bdb::interval<std::int64_t>::literal(
                                100, 120)})
                    .transform([&](bool) { return minted; });
            })
            .and_then([&](std::uint64_t minted) {
                return tx.insert(PayPeriod,
                    PayPeriodRow{.year = minted,
                        .seq = 1,
                        .span = bdb::interval<std::int64_t>::literal(
                            100, 110)});
            });
        if (!landed.has_value()) {
            return std::unexpected{std::move(landed).error()};
        }
        return bdb::commit();
    });
    results.push_back(CaseResult{
        .name = "a holed fiscal year is commit-rejected (no gaps in the "
                "source span)",
        .passed = !holed.has_value()
            && holed.error().kind() == bdb::ErrorKind::CommitRejected
            && !holed.error().violations().empty(),
    });

    auto code = std::error_code{};
    std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
    auto const arguments =
        std::span{argv, static_cast<std::size_t>(argc)};
    if (arguments.size() < 2) {
        std::println("FAIL: usage: r16_payroll <fixtures-file>");
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
