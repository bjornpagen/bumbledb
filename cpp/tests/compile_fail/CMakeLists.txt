# The expected-failure compiler harness (TODO_CPP §34; see README.md).
#
# Each case is an EXCLUDE_FROM_ALL object library — a real project target on
# the real module graph, so cases can import project modules — that the main
# build never compiles. Its CTest test re-enters the build for just that
# target; PASS_REGULAR_EXPRESSION means the test passes ONLY when the build
# output carries the pinned diagnostic substring, i.e. only when the compiler
# rejects the case for the pinned reason. A case that starts compiling
# produces no diagnostic and fails; a diagnostic that degrades stops
# matching and fails.
#
# RUN_SERIAL: every case's test drives ninja in this one binary dir; two
# concurrent `cmake --build` invocations would contend for the ninja lock.
function(bdb_compile_fail_case case)
    set(diagnostic ${ARGN})
    add_library(cf_${case} OBJECT EXCLUDE_FROM_ALL ${case}.cpp)
    target_link_libraries(cf_${case}
        PRIVATE
            bumbledb_version
            bumbledb_language_profile
    )
    add_test(NAME compile_fail.${case}
        COMMAND ${CMAKE_COMMAND} --build ${CMAKE_BINARY_DIR} --target cf_${case}
    )
    set_tests_properties(compile_fail.${case} PROPERTIES
        PASS_REGULAR_EXPRESSION "${diagnostic}"
        RUN_SERIAL TRUE
    )
endfunction()

bdb_compile_fail_case(static_assert_rejected
    "bumbledb compile-fail harness seed: this translation unit is intentionally rejected"
)

bdb_compile_fail_case(unresolved_import
    "bumbledb.nonexistent"
)
