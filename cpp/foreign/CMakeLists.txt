# foreign/: the Rust bridge staticlib plus the quarantine module allowed to
# see the generated C ABI header (TODO_CPP §31/§32).

# ------------------------------------------------------------------------------
# 1. The bridge artifact: cargo builds cpp/bridge into THIS preset's binary
#    dir (separate --target-dir per preset so graphs never collide). The
#    custom target intentionally runs on every build: cargo is its own fast
#    no-op dependency tracker, so bridge sources are never globbed or listed
#    here — cargo owns that graph.
# ------------------------------------------------------------------------------
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

# ------------------------------------------------------------------------------
# 2. The quarantine module `bumbledb_foreign`: one importable named module,
#    internals as partitions. :abi (bridge.cc) is the ONE translation unit
#    allowed to include the generated C header; :raii (raii.cc) is the safe
#    ownership surface over it. No reflection, so the module belongs to BOTH
#    graphs; the lint graph's foreign-zone exclusion (.clang-tidy
#    ExcludeHeaderFilterRegex) covers the included C header.
# ------------------------------------------------------------------------------
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
    # Pinned LLVM 22 quirk: misc-unused-using-decls does not recognize an
    # exported using-declaration as a named module's export surface, so every
    # `export using ::bdb_*;` function re-export in bridge.cc is a false
    # positive ("unused"). Scope the accommodation to exactly this check on
    # exactly this target; every other check still runs over the TUs. Re-test
    # on any clang-tidy bump.
    set_target_properties(bumbledb_foreign PROPERTIES
        CXX_CLANG_TIDY "${CMAKE_CXX_CLANG_TIDY};--checks=-misc-unused-using-decls"
    )

    # Foreign-zone TU (AGENTS.md): raw ABI adaptation is its subject, so the
    # dialect's raw-pointer/void* checks would flag it by design. It stays
    # Clang-COMPILED in the lint graph but is not tidied; enforcement here is
    # review under the foreign boundary rules, exactly like the bridge smoke
    # test.
    set_source_files_properties(raii.cc PROPERTIES SKIP_LINTING ON)
endif()

# ------------------------------------------------------------------------------
# 3. The program-view partition: presents compile-time query IR values as
#    static bdb_program view graphs and marshals execute-time params
#    (TODO_CPP §13, §20-§21). Quarantine code by nature (interior view
#    pointers are its subject), so the FILE lives here — but it is a
#    partition of MODULE `bumbledb` (:foreign_program), added to that target,
#    because it consumes the query IR partitions and a partition of
#    bumbledb_foreign could not import them without a module cycle
#    (bumbledb's partitions import bumbledb_foreign). GCC-only with the rest
#    of the bumbledb module.
# ------------------------------------------------------------------------------
if(NOT BDB_LINT_GRAPH)
    target_sources(bumbledb
        PUBLIC
            FILE_SET CXX_MODULES
            FILES
                program.cc
    )
endif()
