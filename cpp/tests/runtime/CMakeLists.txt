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
