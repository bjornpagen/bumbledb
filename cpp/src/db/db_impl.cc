// Module IMPLEMENTATION unit of `bumbledb` — the one interface/impl split
// in the module, forced by a pinned GCC 16.1 quirk, not by design: a
// NON-template member function DEFINITION whose body instantiates the
// foreign std::expected API corrupts the :db partition's BMI for
// re-export — the primary interface's `export import :db;` then dies with
// "failed to read compiled module cluster N: Bad file data". Template
// members are unaffected. An implementation unit produces no BMI, so the
// bodies of Db::admit, the pre-schema Db::create/open/ephemeral lanes,
// and Db::fingerprint live here (declared in db.cc). Re-test on any GCC
// bump; fold these back into db.cc when the re-export streams clean.
module bumbledb;

import bumbledb_foreign;

namespace bdb {

auto Db::admit(
    std::expected<foreign::db_handle, foreign::error_handle> opened,
    foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
    return std::move(opened)
        .transform([&spec](foreign::db_handle handle) {
            return Db{std::move(handle),
                detail::Manifest{
                    .relation_names = foreign::relation_names_of(spec),
                    .statement_rows = {},
                }};
        })
        .transform_error([](foreign::error_handle handle) {
            return detail::lift(std::move(handle));
        });
}

auto Db::create(std::string_view path,
    foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
    return admit(foreign::db_handle::create(path, spec), spec);
}

auto Db::open(std::string_view path,
    foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
    return admit(foreign::db_handle::open(path, spec), spec);
}

auto Db::ephemeral(std::string_view path,
    foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
    return admit(foreign::db_handle::ephemeral(path, spec), spec);
}

auto Db::fingerprint() const -> std::expected<std::string, Error> {
    return handle_.fingerprint().transform_error(
        [](foreign::error_handle handle) {
            return detail::lift(std::move(handle));
        });
}

} // namespace bdb
