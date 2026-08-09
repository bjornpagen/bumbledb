# Every runtime test imports the one SDK module `bumbledb` (this directory
# is added only in the production graph — the module's reflective
# partitions are GCC-only).
#
# Pinned GCC 16.1 quirk (TODO_CPP §32): `template for` over a >1-element
# range trips -Wshadow on compiler-generated scoping, and expansion happens
# at the template INSTANTIATION site — hence the scoped -Wno-shadow on each
# test target that instantiates the reflective templates, never an inline
# suppression.

add_executable(bumbledb_version_tests)

target_sources(bumbledb_version_tests
    PRIVATE
        version.cc
)

target_link_libraries(bumbledb_version_tests
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

add_test(NAME runtime.version COMMAND bumbledb_version_tests)

add_executable(bumbledb_types_tests)

target_sources(bumbledb_types_tests
    PRIVATE
        types.cc
)

target_link_libraries(bumbledb_types_tests
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

add_test(NAME runtime.types COMMAND bumbledb_types_tests)

add_executable(bumbledb_answers_raw_tests)

target_sources(bumbledb_answers_raw_tests
    PRIVATE
        answers_raw.cc
)

target_link_libraries(bumbledb_answers_raw_tests
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

add_test(NAME runtime.answers_raw COMMAND bumbledb_answers_raw_tests)

add_executable(bumbledb_relation_facade_tests)

target_sources(bumbledb_relation_facade_tests
    PRIVATE
        relation_facade.cc
)

target_link_libraries(bumbledb_relation_facade_tests
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_relation_facade_tests
    PRIVATE -Wno-shadow)

add_test(NAME runtime.meta_relation
    COMMAND bumbledb_relation_facade_tests)

add_executable(bumbledb_schema_law_tests)

target_sources(bumbledb_schema_law_tests
    PRIVATE
        schema_law.cc
)

target_link_libraries(bumbledb_schema_law_tests
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_schema_law_tests
    PRIVATE -Wno-shadow)

add_test(NAME runtime.meta_schema COMMAND bumbledb_schema_law_tests)

add_executable(bumbledb_row_marshal_tests)

target_sources(bumbledb_row_marshal_tests
    PRIVATE
        row_marshal.cc
)

target_link_libraries(bumbledb_row_marshal_tests
    PRIVATE
        bumbledb
        bumbledb_foreign
        bumbledb_language_profile
)

target_compile_options(bumbledb_row_marshal_tests
    PRIVATE -Wno-shadow)

add_test(NAME runtime.meta_row COMMAND bumbledb_row_marshal_tests)

add_executable(bumbledb_db_lifecycle_tests)

target_sources(bumbledb_db_lifecycle_tests
    PRIVATE
        db_lifecycle.cc
)

target_link_libraries(bumbledb_db_lifecycle_tests
    PRIVATE
        bumbledb
        bumbledb_foreign
        bumbledb_language_profile
)

target_compile_options(bumbledb_db_lifecycle_tests
    PRIVATE -Wno-shadow)

add_test(NAME runtime.db_lifecycle COMMAND bumbledb_db_lifecycle_tests)

add_executable(bumbledb_keyed_get_tests)

target_sources(bumbledb_keyed_get_tests
    PRIVATE
        keyed_get.cc
)

target_link_libraries(bumbledb_keyed_get_tests
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_keyed_get_tests
    PRIVATE -Wno-shadow)

add_test(NAME runtime.keyed_get COMMAND bumbledb_keyed_get_tests)

add_executable(bumbledb_query_ir_tests)

target_sources(bumbledb_query_ir_tests
    PRIVATE
        query_ir.cc
)

target_link_libraries(bumbledb_query_ir_tests
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

# -Wno-missing-field-initializers: a query pattern's UNMENTIONED slot
# IS the wildcard (absence of a field binds nothing — lowering.md
# §4.1), so query-writing TUs suppress exactly this -Wextra lint, the
# scoped way (-Wno-shadow's precedent above).
target_compile_options(bumbledb_query_ir_tests
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME runtime.query_ir COMMAND bumbledb_query_ir_tests)
