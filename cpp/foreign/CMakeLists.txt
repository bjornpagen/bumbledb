# foreign/: the Rust bridge staticlib plus the one wrapper module allowed to
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
# 2. The wrapper module: the quarantine TU. It contains no reflection, so it
#    belongs to BOTH graphs; the lint graph's foreign-zone exclusion
#    (.clang-tidy ExcludeHeaderFilterRegex) covers the included C header.
# ------------------------------------------------------------------------------
add_library(bumbledb_foreign)

target_sources(bumbledb_foreign
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            bridge.cppm
)

target_link_libraries(bumbledb_foreign
    PUBLIC
        bumbledb_language_profile
        bumbledb_bridge
)

if(BDB_LINT_GRAPH)
    # Pinned LLVM 22 quirk: misc-unused-using-decls does not recognize an
    # exported using-declaration as a named module's export surface, so every
    # `export using ::bdb_*;` function re-export above is a false positive
    # ("unused"). Scope the accommodation to exactly this check on exactly
    # this target; every other check still runs over the TU. Re-test on any
    # clang-tidy bump.
    set_target_properties(bumbledb_foreign PROPERTIES
        CXX_CLANG_TIDY "${CMAKE_CXX_CLANG_TIDY};--checks=-misc-unused-using-decls"
    )
endif()

# ------------------------------------------------------------------------------
# 3. The RAII adaptation module: owning handles, std::expected outcomes, and
#    the lexical callback trampolines over the raw ABI (TODO_CPP §15-§19,
#    §27). Quarantine code by nature — raw pointers and void* context
#    smuggling are its subject — so it lives HERE, exports a dialect-safe
#    surface upward, and belongs to BOTH graphs (no reflection).
# ------------------------------------------------------------------------------
add_library(bumbledb_foreign_raii)

target_sources(bumbledb_foreign_raii
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            raii.cppm
)

target_link_libraries(bumbledb_foreign_raii
    PUBLIC
        bumbledb_language_profile
        bumbledb_foreign
)

if(BDB_LINT_GRAPH)
    # Foreign-zone TU (AGENTS.md §5.3/§34): raw ABI adaptation is its
    # subject, so the dialect's raw-pointer/void* checks would flag it by
    # design. It stays Clang-COMPILED in the lint graph but is not tidied;
    # enforcement here is review under the foreign boundary rules, exactly
    # like the bridge smoke test and meta/.
    set_source_files_properties(raii.cppm PROPERTIES SKIP_LINTING ON)
endif()

# ------------------------------------------------------------------------------
# 4. The program-view module: presents compile-time query IR values as
#    static bdb_program view graphs and marshals execute-time params
#    (TODO_CPP §13, §20-§21). Quarantine code by nature (interior view
#    pointers are its subject) — but it imports the reflective meta layer,
#    so it is GCC-only, like bumbledb.db.
# ------------------------------------------------------------------------------
if(NOT BDB_LINT_GRAPH)
    add_library(bumbledb_foreign_program)

    target_sources(bumbledb_foreign_program
        PUBLIC
            FILE_SET CXX_MODULES
            FILES
                program.cppm
    )

    target_link_libraries(bumbledb_foreign_program
        PUBLIC
            bumbledb_language_profile
            bumbledb_types
            bumbledb_foreign
            bumbledb_meta_relation
            bumbledb_meta_schema
            bumbledb_meta_query
    )

    target_compile_options(bumbledb_foreign_program PRIVATE -Wno-shadow)
endif()
