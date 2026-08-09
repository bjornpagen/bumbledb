// §34 / TODO_CPP §11 — the find head answers BOUND variables only: a
// head referencing a variable no atom of the rule positively bound is a
// construction-time wall (the engine's safety refusal stands behind it).
// The rule below binds only `service`; the head asks for `window`.
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

inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;
inline constexpr auto Outage = bdb::relation<"Outage", OutageRow>;

inline constexpr auto Uptime = bdb::schema<"Uptime">(
    Service,
    Outage,

    bdb::contained(
        bdb::on(Outage.service),
        bdb::on(Service.id)
    )
);

inline constexpr auto Broken =
    bdb::query(Uptime).rule([](auto r) consteval {
        auto vars = r.vars(Outage);
        return r
            .match(Outage,
                {
                    .service = vars.service,
                })
            .find({
                .window = vars.window,
            });
    });
