/* PIN(gcc-partition-bmi-expected): implementation unit (no BMI to corrupt) for the Db bodies declared in db.cc */
module bumbledb;

import bumbledb_foreign;

namespace bdb {

auto Db::admit(std::expected<foreign::db_handle, foreign::error_handle> opened, foreign::bdb_schema_spec const& spec)
    -> std::expected<Db, Error> {
	return std::move(opened)
	    .transform([&spec](foreign::db_handle handle) {
		    return Db{std::move(handle), detail::Manifest{
		                                     .relation_names = foreign::relation_names_of(spec),
		                                     .statement_rows = {},
		                                 }};
	    })
	    .transform_error([](foreign::error_handle handle) {
		    return detail::lift(std::move(handle));
	    });
}

auto Db::create(std::string_view path, foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
	return admit(foreign::db_handle::create(path, spec), spec);
}

auto Db::open(std::string_view path, foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
	return admit(foreign::db_handle::open(path, spec), spec);
}

auto Db::ephemeral(std::string_view path, foreign::bdb_schema_spec const& spec) -> std::expected<Db, Error> {
	return admit(foreign::db_handle::ephemeral(path, spec), spec);
}

auto Db::fingerprint() const -> std::expected<std::string, Error> {
	return handle_.fingerprint().transform_error([](foreign::error_handle handle) {
		return detail::lift(std::move(handle));
	});
}

}
