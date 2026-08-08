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
#
# Cases link the reflective module set so they can exercise the relation
# reflector. Pinned GCC 16.1 quirk (TODO_CPP §32): `template for` over a
# >1-element range trips -Wshadow at the instantiation site — cases carry
# the same scoped -Wno-shadow as every reflective TU, so the only diagnostic
# a case emits is the one under test.
function(bdb_compile_fail_case case)
    set(diagnostic ${ARGN})
    add_library(cf_${case} OBJECT EXCLUDE_FROM_ALL ${case}.cpp)
    target_link_libraries(cf_${case}
        PRIVATE
            bumbledb_version
            bumbledb_types
            bumbledb_meta_relation
            bumbledb_language_profile
    )
    target_compile_options(cf_${case} PRIVATE -Wno-shadow)
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

# §21: the value vocabulary's consteval lane — an invalid constant interval
# is a compile error that reads as the violated precondition's name.
bdb_compile_fail_case(interval_empty_literal
    "interval_literal_must_satisfy_lo_less_than_hi"
)

# §34: the relation reflector's pinned diagnostics — each names the
# relation, the field, and the offending type (semantic coordinates, never
# template internals).
bdb_compile_fail_case(relation_float_field
    "relation \"Ratio\": field \"ratio\" has unsupported row type 'float'"
)

bdb_compile_fail_case(relation_pointer_field
    "relation \"Labels\": field \"label\" has unsupported row type 'const char"
)

bdb_compile_fail_case(fresh_on_string_field
    "relation \"Naming\": field \"name\" is marked .* but has type .* fresh is legal on std::uint64_t fields only"
)
