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
            bumbledb_meta_closed
            bumbledb_meta_schema
            bumbledb_meta_query
            bumbledb_language_profile
    )
    # -Wno-missing-field-initializers: a query pattern's unmentioned slot
    # IS the wildcard (lowering.md §4.1) — the same scoped suppression
    # every query-writing TU carries, so the only diagnostic a case emits
    # is the one under test.
    target_compile_options(cf_${case}
        PRIVATE -Wno-shadow -Wno-missing-field-initializers)
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

# §34: the schema elaborator's pinned diagnostics — semantic coordinates,
# never template internals (lowering.md §3.4's one-generator wall, the
# key/face single-relation walls, and schema membership).
bdb_compile_fail_case(schema_two_generators_unified
    "the statements unify two generators into one class — \"Repo.id\" and \"Service.id\""
)

bdb_compile_fail_case(key_spans_two_relations
    "coordinates span two relations — \"Outage.service\" and \"Service.id\" — a key constrains one relation's own rows"
)

bdb_compile_fail_case(schema_foreign_coordinate
    "references coordinate \"Outage.service\" but relation \"Outage\" is not a member of the schema"
)

# §34 / TODO_CPP §11, §21: the query elaborator's pinned walls — the
# cross-class variable-reuse diagnostic names BOTH coordinates and both
# classes; a wrong param name at execute fails the synthesized params
# product's designated init; an unbound find-head variable fails rule
# assembly with the named value-tier wall.
bdb_compile_fail_case(query_var_cross_class_reuse
    "variable \"Outage.service\".*cannot bind coordinate \"Audit.actor\""
)

bdb_compile_fail_case(query_wrong_param_name
    "has no non-static data member named .at."
)

bdb_compile_fail_case(query_find_unbound_variable
    "find_head_variable_is_not_bound_in_this_rule"
)

# §34 / TODO_CPP §8: the closed-vocabulary wall — a handle of one closed
# relation used against a field referencing another fails the match
# binding with a diagnostic naming the handle, both vocabularies, and the
# semantic coordinate.
bdb_compile_fail_case(closed_handle_wrong_vocabulary
    "handle \"Deterministic\" of closed relation \"Kind\" cannot bind coordinate \"Ticket.priority\""
)
