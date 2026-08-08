// §34 / lowering.md §3.4 — the one-generator wall: a containment that
// unifies two fresh generators into one class is a schema-construction
// compile error naming BOTH generator coordinates and the statement (two
// mints cannot share a carrier).
import std;
import bumbledb.types;
import bumbledb.meta.relation;
import bumbledb.meta.schema;

struct RepoRow {
    [[=bdb::fresh]]
    std::uint64_t id;
};

struct ServiceRow {
    [[=bdb::fresh]]
    std::uint64_t id;
};

inline constexpr auto Repo = bdb::relation<"Repo", RepoRow>;
inline constexpr auto Service = bdb::relation<"Service", ServiceRow>;

inline constexpr auto Broken = bdb::schema<"Broken">(
    Repo,
    Service,

    bdb::contained(
        bdb::on(Repo.id),
        bdb::on(Service.id)
    )
);
