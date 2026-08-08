# bumbledb.meta.* — the GCC-only reflection layer (TODO_CPP §4, §6–§7,
# §24). This directory is added only in the GCC production graph — never in
# the Clang lint graph (see the top-level CMakeLists.txt).
#
# Pinned GCC 16.1 quirk (TODO_CPP §32): expansion statements (`template
# for`) trip -Wshadow on compiler-generated scoping whenever the expanded
# range has more than one element. Reflective targets therefore carry a
# scoped -Wno-shadow — never an inline suppression. The same option sits on
# every TU that INSTANTIATES the reflective templates (runtime meta tests,
# compile-fail cases): expansion happens at the instantiation site.

add_library(bumbledb_meta_relation)

target_sources(bumbledb_meta_relation
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            bumbledb.meta.relation.cppm
)

target_link_libraries(bumbledb_meta_relation
    PUBLIC
        bumbledb_language_profile
        bumbledb_types
)

target_compile_options(bumbledb_meta_relation PRIVATE -Wno-shadow)

add_library(bumbledb_meta_schema)

target_sources(bumbledb_meta_schema
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            bumbledb.meta.schema.cppm
)

target_link_libraries(bumbledb_meta_schema
    PUBLIC
        bumbledb_language_profile
        bumbledb_types
        bumbledb_meta_relation
)

target_compile_options(bumbledb_meta_schema PRIVATE -Wno-shadow)

add_library(bumbledb_meta_query)

target_sources(bumbledb_meta_query
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            bumbledb.meta.query.cppm
)

target_link_libraries(bumbledb_meta_query
    PUBLIC
        bumbledb_language_profile
        bumbledb_types
        bumbledb_meta_relation
        bumbledb_meta_schema
)

target_compile_options(bumbledb_meta_query PRIVATE -Wno-shadow)

add_library(bumbledb_meta_row)

target_sources(bumbledb_meta_row
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            bumbledb.meta.row.cppm
)

target_link_libraries(bumbledb_meta_row
    PUBLIC
        bumbledb_language_profile
        bumbledb_meta_relation
        bumbledb_foreign
)

target_compile_options(bumbledb_meta_row PRIVATE -Wno-shadow)
