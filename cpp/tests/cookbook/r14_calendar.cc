// Cookbook recipe 14 — The calendar core (ts/COOKBOOK.md §14): policy as
// schema. Hard rules are pointwise keys (rooms cannot double-book), soft
// rules are the statements you decline to write (people CAN double-book);
// accepting an invitation IS claiming the time — the ψ-selected mirrors
// on BOTH SIDES (`Attendance.where(rsvp: Accepted).id ~ Claim.where(arm:
// Busy).source`) is what TYPES `Claim.source` into the "Attendance.id"
// class; and Busy time lies inside working hours pointwise (the
// ψ-selected multi-column coverage containment).
//
// Gates: the engine fingerprint equals the shared golden (fixtures line
// "r14 <64-hex>"); roomConflicts / personLoad (the Allen-vs-param
// queries) prepare AND answer the recipe's own semantics; the two-sided ψ
// mirrors admits accept+claim and rejects an accept without its claim;
// and the free-time twin (recipe 18's `pack`) coalesces one person's
// Busy spans against the real engine.
//
// argv[1] = the fixtures file path (passed by add_test).
import std;
import bumbledb;

inline constexpr auto Rsvp =
    bdb::closed<"Rsvp", "Accepted", "Tentative", "Declined">();
inline constexpr auto Arm = bdb::closed<"Arm", "Busy", "Ooo">();

struct PersonRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    std::string name;
};

struct RoomRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    std::string name;
};

struct EventRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    bdb::interval<std::int64_t> span;
};

struct AttendanceRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    std::uint64_t event;
    std::uint64_t person;
    bdb::ref_to<Rsvp.id> rsvp;
};

struct ClaimRow {
    std::uint64_t source;
    std::uint64_t person;
    bdb::ref_to<Arm.id> arm;
    bdb::interval<std::int64_t> span;
};

struct BookingRow {
    std::uint64_t room;
    std::uint64_t event;
    bdb::interval<std::int64_t> span;
};

struct WorkHoursRow {
    std::uint64_t person;
    bdb::interval<std::int64_t> hours;
};

inline constexpr auto Person = bdb::relation<"Person", PersonRow>;
inline constexpr auto Room = bdb::relation<"Room", RoomRow>;
inline constexpr auto Event = bdb::relation<"Event", EventRow>;
inline constexpr auto Attendance =
    bdb::relation<"Attendance", AttendanceRow>;
inline constexpr auto Claim = bdb::relation<"Claim", ClaimRow>;
inline constexpr auto Booking = bdb::relation<"Booking", BookingRow>;
inline constexpr auto WorkHours = bdb::relation<"WorkHours", WorkHoursRow>;

inline constexpr auto Calendar = bdb::schema<"Calendar">(
    Rsvp,
    Arm,
    Person,
    Room,
    Event,
    Attendance,
    Claim,
    Booking,
    WorkHours,

    bdb::contained(
        bdb::on(Attendance.event),
        bdb::on(Event.id)
    ),
    bdb::contained(
        bdb::on(Attendance.person),
        bdb::on(Person.id)
    ),
    bdb::contained(
        bdb::on(Attendance.rsvp),
        bdb::on(Rsvp.id)
    ),

    // One RSVP per (event, person).
    bdb::key(Attendance.event, Attendance.person),
    bdb::key(Claim.source),
    bdb::contained(
        bdb::on(Claim.person),
        bdb::on(Person.id)
    ),
    bdb::contained(
        bdb::on(Claim.arm),
        bdb::on(Arm.id)
    ),

    // HARD: rooms cannot double-book — the pointwise key.
    bdb::key(Booking.room, Booking.span),

    // SOFT: people CAN double-book — key(Claim, [person, span]) is simply
    // not declared. Accepting an invitation IS claiming the time — and
    // this is the statement that types Claim.source:
    bdb::mirrors(
        bdb::on(bdb::where(Attendance, {.rsvp = Rsvp.Accepted}),
            Attendance.id),
        bdb::on(bdb::where(Claim, {.arm = Arm.Busy}), Claim.source)
    ),

    // Busy time lies inside working hours, pointwise — coverage rides the
    // target's own key.
    bdb::key(WorkHours.person, WorkHours.hours),
    bdb::contained(
        bdb::on(bdb::where(Claim, {.arm = Arm.Busy}),
            Claim.person, Claim.span),
        bdb::on(WorkHours.person, WorkHours.hours)
    ),
    bdb::contained(
        bdb::on(Booking.room),
        bdb::on(Room.id)
    ),
    bdb::contained(
        bdb::on(Booking.event),
        bdb::on(Event.id)
    )
);

// The recipe's queries — the Allen mask against a bind-time interval.
inline constexpr auto RoomConflicts =
    bdb::query(Calendar).rule([](auto r) consteval {
        auto vars = r.vars(Booking);
        return r
            .match(Booking,
                {
                    .room = vars.room,
                    .span = vars.span,
                })
            .where(bdb::allen_in(vars.span, bdb::allen::intersects,
                bdb::param<"want">()))
            .find({
                .room = vars.room,
                .span = vars.span,
            });
    });

inline constexpr auto PersonLoad =
    bdb::query(Calendar).rule([](auto r) consteval {
        auto vars = r.vars(Claim);
        return r
            .match(Claim,
                {
                    .person = vars.person,
                    .span = vars.span,
                })
            .where(bdb::allen_in(vars.span, bdb::allen::intersects,
                bdb::param<"window">()))
            .find({
                .person = vars.person,
                .span = vars.span,
            });
    });

// The free-time twin (recipe 18's coalescing fold on this schema): one
// answer row per (person, maximal Busy segment).
inline constexpr auto BusyBlocks =
    bdb::query(Calendar).rule([](auto r) consteval {
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
                bdb::pack<"block">(vars.span));
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
        / std::format("bumbledb-cookbook-r14-{:08x}{:08x}", device(),
            device());
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
    std::uint64_t room;
    std::uint64_t standup;
    std::uint64_t review;
};

/// Alice works [0,100); two back-to-back events [10,20) and [20,30) are
/// ACCEPTED (each acceptance mirrored by its Busy claim — the two-sided ψ
/// law demands both in one commit); the room hosts the first event.
auto seed(bdb::Db& db) -> std::optional<SeedIds> {
    using Decision = bdb::WriteDecision<SeedIds, std::monostate>;
    using Result = std::expected<Decision, bdb::Error>;
    auto written = db.write([&](bdb::WriteTx& tx) -> Result {
        auto ids = SeedIds{};
        auto standup_accept = std::uint64_t{0};
        auto review_accept = std::uint64_t{0};
        auto const mint = [&](auto coordinate, std::uint64_t& out)
            -> std::expected<bool, bdb::Error> {
            return tx.alloc(coordinate).transform(
                [&](std::uint64_t minted) {
                    out = minted;
                    return true;
                });
        };
        auto rows_land = mint(Person.id, ids.alice)
            .and_then([&](bool) { return mint(Room.id, ids.room); })
            .and_then([&](bool) { return mint(Event.id, ids.standup); })
            .and_then([&](bool) { return mint(Event.id, ids.review); })
            .and_then([&](bool) {
                return mint(Attendance.id, standup_accept);
            })
            .and_then([&](bool) {
                return mint(Attendance.id, review_accept);
            })
            .and_then([&](bool) {
                return tx.insert(Person,
                    PersonRow{
                        .id = ids.alice, .name = std::string{"alice"}});
            })
            .and_then([&](bool) {
                return tx.insert(Room,
                    RoomRow{.id = ids.room, .name = std::string{"3a"}});
            })
            .and_then([&](bool) {
                return tx.insert(WorkHours,
                    WorkHoursRow{.person = ids.alice,
                        .hours = bdb::interval<std::int64_t>::literal(
                            0, 100)});
            })
            .and_then([&](bool) {
                return tx.insert(Event,
                    EventRow{.id = ids.standup,
                        .span = bdb::interval<std::int64_t>::literal(
                            10, 20)});
            })
            .and_then([&](bool) {
                return tx.insert(Event,
                    EventRow{.id = ids.review,
                        .span = bdb::interval<std::int64_t>::literal(
                            20, 30)});
            })
            .and_then([&](bool) {
                return tx.insert(Attendance,
                    AttendanceRow{.id = standup_accept,
                        .event = ids.standup,
                        .person = ids.alice,
                        .rsvp = Rsvp.Accepted});
            })
            .and_then([&](bool) {
                return tx.insert(Claim,
                    ClaimRow{.source = standup_accept,
                        .person = ids.alice,
                        .arm = Arm.Busy,
                        .span = bdb::interval<std::int64_t>::literal(
                            10, 20)});
            })
            .and_then([&](bool) {
                return tx.insert(Attendance,
                    AttendanceRow{.id = review_accept,
                        .event = ids.review,
                        .person = ids.alice,
                        .rsvp = Rsvp.Accepted});
            })
            .and_then([&](bool) {
                return tx.insert(Claim,
                    ClaimRow{.source = review_accept,
                        .person = ids.alice,
                        .arm = Arm.Busy,
                        .span = bdb::interval<std::int64_t>::literal(
                            20, 30)});
            })
            .and_then([&](bool) {
                return tx.insert(Booking,
                    BookingRow{.room = ids.room,
                        .event = ids.standup,
                        .span = bdb::interval<std::int64_t>::literal(
                            10, 20)});
            });
        if (!rows_land.has_value()) {
            return std::unexpected{std::move(rows_land).error()};
        }
        return bdb::commit(ids);
    });
    if (!written.has_value()
        || !std::holds_alternative<bdb::Committed<SeedIds>>(*written)) {
        return std::nullopt;
    }
    return std::get<bdb::Committed<SeedIds>>(*written).value;
}

auto run_cases(std::string_view fixtures_path,
    std::vector<CaseResult>& results) -> void {
    auto const fixtures = slurp(fixtures_path);
    auto const golden =
        fixtures.has_value() ? golden_of(*fixtures, "r14") : std::nullopt;
    if (!golden.has_value()) {
        results.push_back(CaseResult{
            .name = "fixtures file carries an r14 line", .passed = false});
        return;
    }

    auto const dir = make_store_dir();
    if (!dir.has_value()) {
        results.push_back(CaseResult{
            .name = "r14 store directory", .passed = false});
        return;
    }
    auto db = bdb::Db::ephemeral(dir->native(), Calendar);
    if (!db.has_value()) {
        results.push_back(CaseResult{
            .name = "Db::ephemeral admits Calendar", .passed = false});
        return;
    }

    auto const fingerprint = db->fingerprint();
    results.push_back(CaseResult{
        .name = "r14 fingerprint matches the pinned golden",
        .passed = fingerprint.has_value() && *fingerprint == *golden,
    });

    auto const ids = seed(*db);
    results.push_back(CaseResult{
        .name = "accept+claim pairs, coverage, and the booking commit "
                "(the two-sided ψ mirrors admits the paired write)",
        .passed = ids.has_value(),
    });
    if (!ids.has_value()) {
        return;
    }

    // An acceptance WITHOUT its Busy claim violates the ψ mirrors.
    auto unpaired = db->write([&](bdb::WriteTx& tx)
            -> std::expected<
                bdb::WriteDecision<std::monostate, std::monostate>,
                bdb::Error> {
        auto landed = tx.alloc(Event.id)
            .and_then([&](std::uint64_t event) {
                return tx.insert(Event,
                        EventRow{.id = event,
                            .span = bdb::interval<std::int64_t>::literal(
                                40, 50)})
                    .transform([&](bool) { return event; });
            })
            .and_then([&](std::uint64_t event) {
                return tx.alloc(Attendance.id)
                    .and_then([&](std::uint64_t accept) {
                        return tx.insert(Attendance,
                            AttendanceRow{.id = accept,
                                .event = event,
                                .person = ids->alice,
                                .rsvp = Rsvp.Accepted});
                    });
            });
        if (!landed.has_value()) {
            return std::unexpected{std::move(landed).error()};
        }
        return bdb::commit();
    });
    results.push_back(CaseResult{
        .name = "an acceptance without its Busy claim is commit-rejected",
        .passed = !unpaired.has_value()
            && unpaired.error().kind() == bdb::ErrorKind::CommitRejected
            && !unpaired.error().violations().empty(),
    });

    // A TENTATIVE reply claims nothing — the ψ selection admits it bare.
    auto tentative = db->write([&](bdb::WriteTx& tx)
            -> std::expected<
                bdb::WriteDecision<std::monostate, std::monostate>,
                bdb::Error> {
        auto landed = tx.alloc(Event.id)
            .and_then([&](std::uint64_t event) {
                return tx.insert(Event,
                        EventRow{.id = event,
                            .span = bdb::interval<std::int64_t>::literal(
                                60, 70)})
                    .transform([&](bool) { return event; });
            })
            .and_then([&](std::uint64_t event) {
                return tx.alloc(Attendance.id)
                    .and_then([&](std::uint64_t reply) {
                        return tx.insert(Attendance,
                            AttendanceRow{.id = reply,
                                .event = event,
                                .person = ids->alice,
                                .rsvp = Rsvp.Tentative});
                    });
            });
        if (!landed.has_value()) {
            return std::unexpected{std::move(landed).error()};
        }
        return bdb::commit();
    });
    results.push_back(CaseResult{
        .name = "a Tentative reply commits without a claim (the ψ "
                "selection is the membership rule)",
        .passed = tentative.has_value()
            && std::holds_alternative<bdb::Committed<std::monostate>>(
                *tentative),
    });

    auto room_conflicts = db->prepare<RoomConflicts>();
    auto person_load = db->prepare<PersonLoad>();
    auto busy_blocks = db->prepare<BusyBlocks>();
    results.push_back(CaseResult{
        .name = "roomConflicts / personLoad / busyBlocks prepare through "
                "the engine validator",
        .passed = room_conflicts.has_value() && person_load.has_value()
            && busy_blocks.has_value(),
    });
    if (!room_conflicts.has_value() || !person_load.has_value()
        || !busy_blocks.has_value()) {
        return;
    }

    // roomConflicts: [15,25) intersects the [10,20) booking; [20,30)
    // touches it only at the boundary (meets shares no point).
    auto conflicts = db->execute(*room_conflicts,
        {.want = bdb::interval<std::int64_t>::literal(15, 25)});
    results.push_back(CaseResult{
        .name = "roomConflicts([15,25)) answers {(3a, [10,20))}",
        .passed = conflicts.has_value() && conflicts->size() == 1
            && conflicts->rows().front().room == ids->room
            && conflicts->rows().front().span
                == bdb::interval<std::int64_t>::literal(10, 20),
    });
    auto clear = db->execute(*room_conflicts,
        {.want = bdb::interval<std::int64_t>::literal(20, 30)});
    results.push_back(CaseResult{
        .name = "roomConflicts([20,30)) answers the empty set (meets "
                "shares no point)",
        .passed = clear.has_value() && clear->size() == 0,
    });

    // personLoad: both claims intersect [0,50).
    auto load = db->execute(*person_load,
        {.window = bdb::interval<std::int64_t>::literal(0, 50)});
    results.push_back(CaseResult{
        .name = "personLoad([0,50)) answers alice's two claims",
        .passed = load.has_value() && load->size() == 2,
    });

    // The coalescing fold: [10,20) and [20,30) pack to ONE [10,30) block.
    auto blocks = db->execute(*busy_blocks, {});
    results.push_back(CaseResult{
        .name = "busyBlocks packs [10,20)+[20,30) to one [10,30) block",
        .passed = blocks.has_value() && blocks->size() == 1
            && blocks->rows().front().person == ids->alice
            && blocks->rows().front().block
                == bdb::interval<std::int64_t>::literal(10, 30),
    });

    auto code = std::error_code{};
    std::filesystem::remove_all(*dir, code);
}

} // namespace

auto main(int argc, char** argv) -> int {
    auto const arguments =
        std::span{argv, static_cast<std::size_t>(argc)};
    if (arguments.size() < 2) {
        std::println("FAIL: usage: r14_calendar <fixtures-file>");
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
