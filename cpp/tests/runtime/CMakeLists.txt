add_executable(bumbledb_version_tests)

target_sources(bumbledb_version_tests
    PRIVATE
        version.test.cpp
)

target_link_libraries(bumbledb_version_tests
    PRIVATE
        bumbledb_version
        bumbledb_language_profile
)

add_test(NAME runtime.version COMMAND bumbledb_version_tests)

add_executable(bumbledb_types_tests)

target_sources(bumbledb_types_tests
    PRIVATE
        types.test.cpp
)

target_link_libraries(bumbledb_types_tests
    PRIVATE
        bumbledb_types
        bumbledb_language_profile
)

add_test(NAME runtime.types COMMAND bumbledb_types_tests)

add_executable(bumbledb_answers_raw_tests)

target_sources(bumbledb_answers_raw_tests
    PRIVATE
        answers_raw.test.cpp
)

target_link_libraries(bumbledb_answers_raw_tests
    PRIVATE
        bumbledb_answers
        bumbledb_language_profile
)

if(BDB_LINT_GRAPH)
    # The §36 inert-moved-from-source property IS this test's subject, so
    # bugprone-use-after-move flags it by design. The TU stays
    # Clang-COMPILED in the lint graph but is not tidied (the abi_smoke
    # precedent); enforcement is review.
    set_source_files_properties(answers_raw.test.cpp
        PROPERTIES SKIP_LINTING ON)
endif()

add_test(NAME runtime.answers_raw COMMAND bumbledb_answers_raw_tests)

# Reflective tests import bumbledb.meta.* and are therefore GCC-only (the
# meta modules do not exist in the lint graph). Pinned GCC 16.1 quirk
# (TODO_CPP §32): `template for` over a >1-element range trips -Wshadow on
# compiler-generated scoping, and expansion happens HERE, at the template
# instantiation site — hence the scoped -Wno-shadow on each reflective test
# target, never an inline suppression.
if(NOT BDB_LINT_GRAPH)
    add_executable(bumbledb_relation_facade_tests)

    target_sources(bumbledb_relation_facade_tests
        PRIVATE
            relation_facade.test.cpp
    )

    target_link_libraries(bumbledb_relation_facade_tests
        PRIVATE
            bumbledb_types
            bumbledb_meta_relation
            bumbledb_language_profile
    )

    target_compile_options(bumbledb_relation_facade_tests
        PRIVATE -Wno-shadow)

    add_test(NAME runtime.meta_relation
        COMMAND bumbledb_relation_facade_tests)

    add_executable(bumbledb_row_marshal_tests)

    target_sources(bumbledb_row_marshal_tests
        PRIVATE
            row_marshal.test.cpp
    )

    target_link_libraries(bumbledb_row_marshal_tests
        PRIVATE
            bumbledb_types
            bumbledb_meta_row
            bumbledb_foreign
            bumbledb_language_profile
    )

    target_compile_options(bumbledb_row_marshal_tests
        PRIVATE -Wno-shadow)

    add_test(NAME runtime.meta_row COMMAND bumbledb_row_marshal_tests)

    add_executable(bumbledb_db_lifecycle_tests)

    target_sources(bumbledb_db_lifecycle_tests
        PRIVATE
            db_lifecycle.test.cpp
    )

    target_link_libraries(bumbledb_db_lifecycle_tests
        PRIVATE
            bumbledb_umbrella
            bumbledb_foreign
            bumbledb_foreign_raii
            bumbledb_language_profile
    )

    target_compile_options(bumbledb_db_lifecycle_tests
        PRIVATE -Wno-shadow)

    add_test(NAME runtime.db_lifecycle COMMAND bumbledb_db_lifecycle_tests)
endif()
