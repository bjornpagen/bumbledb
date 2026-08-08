# The cookbook parity suite (TODO_CPP §33): each recipe builds its theory
# through the real bdb::schema<> elaborator, creates a store, and asserts
# the engine's fingerprint readback equals the host-neutral golden at the
# repository root — the SAME fixtures file the TypeScript suite reads
# (fixtures/cookbook-fingerprints.txt), passed as a test argument.
#
# GCC-only (the schema elaborator's import graph reaches meta/) — this
# directory is added only in the production graph.

add_executable(bumbledb_cookbook_r01)

target_sources(bumbledb_cookbook_r01
    PRIVATE
        r01_uptime.test.cpp
)

target_link_libraries(bumbledb_cookbook_r01
    PRIVATE
        bumbledb_umbrella
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r01 PRIVATE -Wno-shadow)

add_test(NAME cookbook.r01_uptime
    COMMAND bumbledb_cookbook_r01
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

# Recipe 1's QUERIES against the real engine (TODO_CPP §11-§12, §20-§23):
# prepare through the engine validator, execute, assert the recipe's own
# answers. -Wno-missing-field-initializers: an unmentioned query-pattern
# slot IS the wildcard (lowering.md §4.1), so query-writing TUs suppress
# exactly this -Wextra lint, the scoped way (-Wno-shadow's precedent).
add_executable(bumbledb_cookbook_r01_queries)

target_sources(bumbledb_cookbook_r01_queries
    PRIVATE
        r01_queries.test.cpp
)

target_link_libraries(bumbledb_cookbook_r01_queries
    PRIVATE
        bumbledb_umbrella
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r01_queries
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r01_queries COMMAND bumbledb_cookbook_r01_queries)
