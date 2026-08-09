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
    # This is the ONE test TU that holds raw ABI handles (its subject IS the
    # raw boundary — TODO_CPP §35), so the dialect's raw-pointer checks would
    # flag it by design. The foreign-zone exclusion in .clang-tidy covers only
    # the included header, not this main file; it stays Clang-COMPILED in the
    # lint graph but is not tidied. Enforcement here is review under the
    # foreign boundary rules, exactly like foreign/ (AGENTS.md §34).
    set_source_files_properties(abi_smoke.cc PROPERTIES SKIP_LINTING ON)
endif()
