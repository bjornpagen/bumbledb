add_executable(bumbledb_abi_smoke_tests)

target_sources(bumbledb_abi_smoke_tests
    PRIVATE
        abi_smoke.cc
)

target_link_libraries(bumbledb_abi_smoke_tests
    PRIVATE
        bumbledb_foreign
        bumbledb_language_profile
)

add_test(NAME bridge.abi_smoke COMMAND bumbledb_abi_smoke_tests)

if(BDB_LINT_GRAPH)
    # The one test TU whose subject is the raw boundary: Clang-compiled but untidied; enforcement is review under the foreign boundary rules.
    set_source_files_properties(abi_smoke.cc PROPERTIES SKIP_LINTING ON)
endif()
