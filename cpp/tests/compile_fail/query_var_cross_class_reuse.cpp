// §34 / TODO_CPP §11 — the query cross-class wall: a variable minted in
// one law class refuses to bind a column of another (two physical u64
// columns are not query-compatible merely because both are uint64_t).
// The diagnostic names BOTH coordinates and both classes.
import std;
import bumbledb.types;
import bumbledb.meta.relation;
import bumbledb.meta.schema;
import bumbledb.meta.query;

struct ServiceRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    std::string name;
};

struct OutageRow {
    std::uint64_t service;
    bdb::interval<std::int64_t> window;
};

// A schema-less second relation: `actor` is a bare u64 — no law touches
// it, so it shares no class with Outage.service (class "Service.id").
struct AuditRow {
    std::uint64_t actor;
};

inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;
inline constexpr auto Outage = bdb::relation<"Outage", OutageRow>;
inline constexpr auto Audit = bdb::relation<"Audit", AuditRow>;

inline constexpr auto Uptime = bdb::schema<"Uptime">(
    Service,
    Outage,
    Audit,

    bdb::contained(
        bdb::on(Outage.service),
        bdb::on(Service.id)
    )
);

inline constexpr auto Broken =
    bdb::query(Uptime).rule([](auto r) consteval {
        auto outage = r.vars(Outage);
        return r
            .match(Outage,
                {
                    .service = outage.service,
                    .window = outage.window,
                })
            .match(Audit,
                {
                    .actor = outage.service,
                })
            .find({
                .service = outage.service,
            });
    });
