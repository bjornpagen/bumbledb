# runtime/ and cookbook/ import the reflective `bumbledb` module (GCC-only);
# bridge/ imports only bumbledb_foreign. This directory is production-graph
# only — the lint graph does not add it.

add_subdirectory(runtime)
add_subdirectory(bridge)
add_subdirectory(cookbook)

add_executable(bumbledb_conformance_tests)
target_sources(bumbledb_conformance_tests PRIVATE conformance.cc)
target_link_libraries(bumbledb_conformance_tests PRIVATE bumbledb_language_profile)
# PIN(gcc-template-for-wshadow): expansion statements in the conformance TU
set_source_files_properties(conformance.cc PROPERTIES COMPILE_OPTIONS -Wno-shadow)
add_test(NAME runtime.conformance COMMAND bumbledb_conformance_tests)

# PIN(compile-fail-vs-sanitizers): instrumented graphs cannot reproduce the pinned diagnostics; the suite drives the production compiler only.
if(NOT CMAKE_CXX_FLAGS MATCHES "-fsanitize")
    add_subdirectory(compile_fail)
endif()

if(BDB_DARWIN)
    set_tests_properties(runtime.conformance PROPERTIES ENVIRONMENT "MallocProbGuard=1")
endif()
