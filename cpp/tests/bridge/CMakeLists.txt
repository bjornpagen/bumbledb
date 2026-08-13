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

if(BDB_DARWIN)
    set_tests_properties(bridge.abi_smoke PROPERTIES ENVIRONMENT "MallocProbGuard=1")
endif()
