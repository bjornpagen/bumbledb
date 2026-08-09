// Keyed reads through first-class law values (TODO_CPP §26) over the
// schema-typed Db lane (§13): Db::ephemeral(path, schema-value), the
// stored key law as the get() selector (structural identity — no
// generated nominal type), the fresh-field primary lane, the WriteTx
// final-state twin, and miss-as-absence. Also proves the capacity and
// mirrors wire paths end-to-end: the engine ADMITS the schemas this
// elaborator lowers. GCC-only (the `bumbledb` module's reflective graph).
import std;
import bumbledb;

namespace {

struct CaseResult {
    std::string name;
    bool passed;
};

// TODO_CPP §39 — the Uptime theory through the real elaborator.
struct ServiceRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    std::string name;
};

struct OutageRow {
    std::uint64_t service;
    bdb::interval<std::int64_t> window;
};

inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;
inline constexpr auto Outage = bdb::relation<"Outage", OutageRow>;

// §26: the law is stored once and passed back verbatim — it IS the
// selector.
inline constexpr auto outage_key = bdb::key(Outage.service, Outage.window);

inline constexpr auto Uptime = bdb::schema<"Uptime">(
    Service,
    Outage,

    bdb::contained(
        bdb::on(Outage.service),
        bdb::on(Service.id)
    ),

    outage_key
);

using UnitDecision = bdb::WriteDecision<std::monostate, std::monostate>;
using UnitResult = std::expected<UnitDecision, bdb::Error>;

auto make_store_dir(std::string_view label)
    -> std::optional<std::filesystem::path> {
    auto code = std::error_code{};
    auto const root = std::filesystem::temp_directory_path(code);
    if (code) {
        return std::nullopt;
    }
    auto device = std::random_device{};
    auto const dir = root
        / std::format("bumbledb-keyed-get-{}-{:08x}{:08x}", label,
            device(), device());
    std::filesystem::remove_all(dir, code);
    code.clear();
    std::filesystem::create_directories(dir, code);
    if (code) {
        return std::nullopt;
    }
    return dir;
}

auto cell_is_u64(bdb::RowSet const& rows, bdb::Cell at, std::uint64_t want)
    -> bool {
    auto const cell = rows.cell(at);
    return cell.has_value()
        && std::holds_alternative<std::uint64_t>(*cell)
        && std::get<std::uint64_t>(*cell) == want;
}

auto cell_is_text(bdb::RowSet const& rows, bdb::Cell at,
    std::string_view want) -> bool {
    auto const cell = rows.cell(at);
    return cell.has_value()
        && std::holds_alternative<std::string_view>(*cell)
        && std::get<std::string_view>(*cell) == want;
}

auto run_uptime_cases(std::vector<CaseResult>& results) -> void {
    auto const dir = make_store_dir("uptime");
    if (!dir.has_value()) {
        results.push_back(CaseResult{
            .name = "keyed-get store directory", .passed = false});
        return;
    }
    auto db = bdb::Db::ephemeral(dir->native(), Uptime);
    results.push_back(CaseResult{
        .name = "Db::ephemeral admits the schema-typed Uptime theory (§13)",
        .passed = db.has_value(),
    });
    if (!db.has_value()) {
        return;
    }

    auto const window = bdb::interval<std::int64_t>::literal(0, 100);

    // Seed one Service + one Outage, carrying the fresh id out.
    using IdDecision = bdb::WriteDecision<std::uint64_t, std::monostate>;
    using IdResult = std::expected<IdDecision, bdb::Error>;
    auto written = db->write([&](bdb::WriteTx& tx) -> IdResult {
        return tx.alloc(Service.id)
            .and_then([&](std::uint64_t id) -> IdResult {
                return tx
                    .insert(Service,
                        ServiceRow{.id = id, .name = std::string{"search"}})
                    .and_then([&](bool) {
                        return tx.insert(Outage,
                            OutageRow{.service = id, .window = window});
                    })
                    .transform([id](bool) -> IdDecision {
                        return bdb::commit(id);
                    });
            });
    });
    auto const committed = written.has_value()
        && std::holds_alternative<bdb::Committed<std::uint64_t>>(*written);
    results.push_back(CaseResult{
        .name = "seed write commits Service + Outage",
        .passed = committed,
    });
    if (!committed) {
        return;
    }
    auto const id = std::get<bdb::Committed<std::uint64_t>>(*written).value;

    // §26: the fresh-field primary lane — db.get(Service, {.id = id}).
    auto const primary = db->get(Service, {.id = id});
    results.push_back(CaseResult{
        .name = "db.get(Service, {.id}) reads through the fresh primary key",
        .passed = primary.has_value() && primary->has_value()
            && (*primary)->len() == 1
            && cell_is_u64(**primary, {.row = 0, .column = 0}, id)
            && cell_is_text(**primary, {.row = 0, .column = 1}, "search"),
    });

    // §26: the stored law as the selector — snap.get through db.get.
    auto const keyed =
        db->get(Outage, outage_key, {.service = id, .window = window});
    results.push_back(CaseResult{
        .name = "db.get(Outage, outage_key, pattern) resolves the law "
                "structurally",
        .passed = keyed.has_value() && keyed->has_value()
            && (*keyed)->len() == 1
            && cell_is_u64(**keyed, {.row = 0, .column = 0}, id),
    });

    // A key miss is genuine absence, not an error.
    auto const missing_window =
        bdb::interval<std::int64_t>::literal(500, 600);
    auto const miss = db->get(
        Outage, outage_key, {.service = id, .window = missing_window});
    results.push_back(CaseResult{
        .name = "a keyed miss is nullopt (absence, never an error)",
        .passed = miss.has_value() && !miss->has_value(),
    });

    // The Snapshot lane spells identically inside a read.
    auto const snap_hit = db->read(
        [&](bdb::Snapshot& snap) -> std::expected<bool, bdb::Error> {
            return snap
                .get(Outage, outage_key,
                    {.service = id, .window = window})
                .transform([&](std::optional<bdb::RowSet> rows) {
                    return rows.has_value() && rows->len() == 1;
                });
        });
    results.push_back(CaseResult{
        .name = "snap.get sees the committed row",
        .passed = snap_hit.has_value() && *snap_hit,
    });

    // The WriteTx twin reads the FINAL state: a delta insert is visible
    // to tx.get before commit; the write is then abandoned and the fact
    // never lands.
    auto const pending_window =
        bdb::interval<std::int64_t>::literal(200, 300);
    auto observed = db->write([&](bdb::WriteTx& tx) -> UnitResult {
        return tx
            .insert(Outage,
                OutageRow{.service = id, .window = pending_window})
            .and_then([&](bool) {
                return tx.get(Outage, outage_key,
                    {.service = id, .window = pending_window});
            })
            .transform([](std::optional<bdb::RowSet> rows) -> UnitDecision {
                return rows.has_value() && rows->len() == 1
                    ? UnitDecision{bdb::abandon()}
                    : UnitDecision{bdb::commit()};
            });
    });
    auto const tx_saw_pending = observed.has_value()
        && std::holds_alternative<bdb::Abandoned<std::monostate>>(
            *observed);
    auto const dropped = db->get(
        Outage, outage_key, {.service = id, .window = pending_window});
    results.push_back(CaseResult{
        .name = "tx.get reads the final state; the abandoned delta drops",
        .passed = tx_saw_pending && dropped.has_value()
            && !dropped->has_value(),
    });
}

// The capacity and mirrors wire paths: the engine admits what the
// elaborator lowers (weigh/within/ref and one bidirectional statement).
struct PoolRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    std::uint64_t supply;
};

struct DeviceRow {
    std::uint64_t pool;
    std::uint64_t watts;
};

inline constexpr auto Pool = bdb::relation<"Pool", PoolRow>;
inline constexpr auto Device = bdb::relation<"Device", DeviceRow>;

inline constexpr auto Power = bdb::schema<"Power">(
    Pool,
    Device,

    bdb::contained(
        bdb::on(Device.pool),
        bdb::on(Pool.id)
    ),

    bdb::capacity(
        bdb::on(Pool.id),
        bdb::weigh(Device.watts),
        bdb::within(std::uint64_t{0}, bdb::ref(Pool.supply)),
        bdb::on(Device.pool)
    )
);

struct RoomRow {
    [[=bdb::fresh]]
    std::uint64_t id;
};

struct BookingRow {
    std::uint64_t room;
    bdb::interval<std::uint64_t> span;
};

inline constexpr auto Room = bdb::relation<"Room", RoomRow>;
inline constexpr auto Booking = bdb::relation<"Booking", BookingRow>;

// The mirrors bijection needs BOTH directions key-backed at engine
// validation: Room.id is the fresh-implied key; Booking.room gets a
// declared key.
inline constexpr auto Rooms = bdb::schema<"Rooms">(
    Room,
    Booking,

    bdb::key(Booking.room),

    bdb::mirrors(
        bdb::on(Booking.room),
        bdb::on(Room.id)
    ),

    bdb::capacity(
        bdb::on(Room.id),
        bdb::weigh(bdb::duration(Booking.span)),
        bdb::within(std::uint64_t{0}, std::uint64_t{720}),
        bdb::on(Booking.room)
    )
);

auto run_admission_cases(std::vector<CaseResult>& results) -> void {
    auto const power_dir = make_store_dir("power");
    auto const rooms_dir = make_store_dir("rooms");
    if (!power_dir.has_value() || !rooms_dir.has_value()) {
        results.push_back(CaseResult{
            .name = "admission store directories", .passed = false});
        return;
    }
    auto power = bdb::Db::ephemeral(power_dir->native(), Power);
    results.push_back(CaseResult{
        .name = "the engine admits a weigh/within/ref capacity statement",
        .passed = power.has_value(),
    });
    auto rooms = bdb::Db::ephemeral(rooms_dir->native(), Rooms);
    results.push_back(CaseResult{
        .name = "the engine admits mirrors + duration-weighed capacity",
        .passed = rooms.has_value(),
    });
}

} // namespace

auto main() -> int {
    auto results = std::vector<CaseResult>{};
    run_uptime_cases(results);
    run_admission_cases(results);

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
