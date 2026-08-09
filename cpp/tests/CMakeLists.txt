# runtime/ and cookbook/ import the reflective `bumbledb` module (GCC-only, production graph); bridge/ imports only bumbledb_foreign, so it belongs to both graphs.
if(NOT BDB_LINT_GRAPH)
    add_subdirectory(runtime)
endif()
add_subdirectory(bridge)

# PIN(compile-fail-vs-sanitizers): instrumented graphs cannot reproduce the pinned diagnostics; the suite drives the production compiler only.
if(NOT BDB_LINT_GRAPH AND NOT CMAKE_CXX_FLAGS MATCHES "-fsanitize")
    add_subdirectory(compile_fail)
endif()

if(NOT BDB_LINT_GRAPH)
    add_subdirectory(cookbook)
endif()
