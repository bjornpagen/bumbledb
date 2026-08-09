# runtime/ and cookbook/ import the `bumbledb` module, which contains
# reflective partitions and is therefore GCC-only — those suites exist only
# in the production graph. bridge/ imports only `bumbledb_foreign` (no
# reflection), so it belongs to both graphs.
if(NOT BDB_LINT_GRAPH)
    add_subdirectory(runtime)
endif()
add_subdirectory(bridge)

# The compile-fail harness drives the production compiler and pins GCC
# diagnostic text (TODO_CPP §32, §34); it is not part of the lint graph.
# It is also not part of the sanitizer graphs: pinned GCC 16.1 quirk —
# -fsanitize=address,undefined breaks constexpr std::string evaluation
# inside the reflector's static_assert message machinery ("is not a
# constant expression" instead of the pinned diagnostic), so instrumented
# graphs cannot reproduce the pinned text. Diagnostics are a product of
# the canonical production configurations (dev/release), which is where
# the suite runs — the §32 CI matrix keeps compile-fail as its own job.
if(NOT BDB_LINT_GRAPH AND NOT CMAKE_CXX_FLAGS MATCHES "-fsanitize")
    add_subdirectory(compile_fail)
endif()

# The cookbook parity suite imports the schema elaborator (a reflective
# partition of `bumbledb`, GCC-only), so it exists only in the production
# graph.
if(NOT BDB_LINT_GRAPH)
    add_subdirectory(cookbook)
endif()
