# The closed value vocabulary (TODO_CPP §7, §21): reflection-free dialect
# code, part of BOTH graphs.
add_library(bumbledb_types)

target_sources(bumbledb_types
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            bumbledb.types.cppm
)

target_link_libraries(bumbledb_types
    PUBLIC
        bumbledb_language_profile
)

# The failure vocabulary (TODO_CPP §19, §27-§28): reflection-free dialect
# code, part of BOTH graphs.
add_library(bumbledb_error)

target_sources(bumbledb_error
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            bumbledb.error.cppm
)

target_link_libraries(bumbledb_error
    PUBLIC
        bumbledb_language_profile
        bumbledb_foreign_raii
)

# The untyped result lane (TODO_CPP §22-§23): reflection-free dialect code,
# part of BOTH graphs.
add_library(bumbledb_answers)

target_sources(bumbledb_answers
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            bumbledb.answers.cppm
)

target_link_libraries(bumbledb_answers
    PUBLIC
        bumbledb_language_profile
        bumbledb_types
        bumbledb_foreign_raii
)

# Host-side answer ordering (ts/src/order.ts): reflection-free dialect
# code, part of BOTH graphs.
add_library(bumbledb_order)

target_sources(bumbledb_order
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            bumbledb.order.cppm
)

target_link_libraries(bumbledb_order
    PUBLIC
        bumbledb_language_profile
        bumbledb_types
)

# The runtime resource layer and the umbrella module are GCC-only: bumbledb.db
# imports the reflection-backed meta marshaller (bumbledb.meta.row), and the
# umbrella re-exports it plus bumbledb.meta.relation. ACCEPTED zoning
# (TODO_CPP §32): the modules contain no reflection syntax themselves, but
# their import graphs reach meta/, which does not exist in the lint graph.
if(NOT BDB_LINT_GRAPH)
    add_library(bumbledb_db)

    target_sources(bumbledb_db
        PUBLIC
            FILE_SET CXX_MODULES
            FILES
                bumbledb.db.cppm
    )

    target_link_libraries(bumbledb_db
        PUBLIC
            bumbledb_language_profile
            bumbledb_error
            bumbledb_answers
            bumbledb_foreign_raii
            bumbledb_foreign_program
            bumbledb_meta_relation
            bumbledb_meta_schema
            bumbledb_meta_row
            bumbledb_meta_query
    )

    add_library(bumbledb_umbrella)

    target_sources(bumbledb_umbrella
        PUBLIC
            FILE_SET CXX_MODULES
            FILES
                bumbledb.cppm
    )

    target_link_libraries(bumbledb_umbrella
        PUBLIC
            bumbledb_language_profile
            bumbledb_types
            bumbledb_error
            bumbledb_answers
            bumbledb_order
            bumbledb_db
            bumbledb_meta_relation
            bumbledb_meta_closed
            bumbledb_meta_schema
            bumbledb_meta_query
    )
endif()

add_library(bumbledb_version)

target_sources(bumbledb_version
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            bumbledb.version.cppm
    PRIVATE
        bumbledb.version.cpp
)

target_link_libraries(bumbledb_version
    PUBLIC
        bumbledb_language_profile
)
