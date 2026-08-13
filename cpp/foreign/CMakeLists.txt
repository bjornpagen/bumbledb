# The bridge custom target intentionally runs on every build: cargo is its own no-op dependency tracker and owns the bridge source graph; per-preset --target-dir keeps graphs from colliding.
find_program(BDB_CARGO cargo REQUIRED)

set(BDB_BRIDGE_TARGET_DIR "${CMAKE_CURRENT_BINARY_DIR}/cargo")
set(BDB_BRIDGE_STATICLIB
    "${BDB_BRIDGE_TARGET_DIR}/release/${CMAKE_STATIC_LIBRARY_PREFIX}bumbledb_cpp_bridge${CMAKE_STATIC_LIBRARY_SUFFIX}"
)

add_custom_target(bumbledb_bridge_cargo
    COMMAND
        "${BDB_CARGO}" build --release
        --manifest-path "${CMAKE_CURRENT_SOURCE_DIR}/../bridge/Cargo.toml"
        --target-dir "${BDB_BRIDGE_TARGET_DIR}"
    BYPRODUCTS "${BDB_BRIDGE_STATICLIB}"
    COMMENT "cargo build --release cpp/bridge (bumbledb-cpp-bridge staticlib)"
    VERBATIM
)

add_library(bumbledb_bridge STATIC IMPORTED GLOBAL)
set_target_properties(bumbledb_bridge PROPERTIES
    IMPORTED_LOCATION "${BDB_BRIDGE_STATICLIB}"
)
add_dependencies(bumbledb_bridge bumbledb_bridge_cargo)

# bumbledb_foreign holds no reflection, so it belongs to both graphs; the lint graph's ExcludeHeaderFilterRegex covers the included C header.
add_library(bumbledb_foreign)

target_sources(bumbledb_foreign
    PUBLIC
        FILE_SET CXX_MODULES
        BASE_DIRS ${PROJECT_SOURCE_DIR}
        FILES
            bumbledb_foreign.cc
            bridge.cc
            raii.cc
)

target_link_libraries(bumbledb_foreign
    PUBLIC
        bumbledb_language_profile
        bumbledb_bridge
)

if(BDB_LINT_GRAPH)
    # PIN(llvm22-unused-using-decls)
    set_target_properties(bumbledb_foreign PROPERTIES
        CXX_CLANG_TIDY "${CMAKE_CXX_CLANG_TIDY};--checks=-misc-unused-using-decls"
    )
endif()

# program.cc is quarantine code but a partition of MODULE bumbledb (:foreign_program): it consumes the query IR partitions, and a bumbledb_foreign partition could not import them without a module cycle.
if(NOT BDB_LINT_GRAPH)
    target_sources(bumbledb
        PUBLIC
            FILE_SET CXX_MODULES
            FILES
                program.cc
    )
endif()
