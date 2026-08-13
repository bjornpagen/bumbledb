# The expected-failure compiler harness: each case is an EXCLUDE_FROM_ALL object library whose CTest test re-enters the build for just that target; PASS_REGULAR_EXPRESSION passes only when the build output carries the pinned diagnostic.
# RUN_SERIAL: every case drives ninja in this one binary dir; concurrent builds would contend for the ninja lock.
function(bdb_compile_fail_case case)
    set(diagnostic ${ARGN})
    add_library(cf_${case} OBJECT EXCLUDE_FROM_ALL ${case}.cc)
    target_link_libraries(cf_${case}
        PRIVATE
            bumbledb
            bumbledb_language_profile
    )
    # PIN(gcc-template-for-wshadow); -Wno-missing-field-initializers: an unmentioned query-pattern slot IS the wildcard (lowering.md 4.1) — so the only diagnostic a case emits is the one under test.
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
    "bumbledb_nonexistent"
)

bdb_compile_fail_case(interval_empty_literal
    "interval_literal_must_satisfy_lo_less_than_hi"
)

bdb_compile_fail_case(relation_float_field
    "relation \"Ratio\": field \"ratio\" has unsupported row type 'float'"
)

bdb_compile_fail_case(relation_pointer_field
    "relation \"Labels\": field \"label\" has unsupported row type 'const char"
)

bdb_compile_fail_case(fresh_on_string_field
    "relation \"Naming\": field \"name\" is marked .* but has type .* fresh is legal on std::uint64_t fields only"
)

bdb_compile_fail_case(schema_two_generators_unified
    "the statements unify two generators into one class — \"Repo.id\" and \"Service.id\""
)

bdb_compile_fail_case(key_spans_two_relations
    "coordinates span two relations — \"Outage.service\" and \"Service.id\" — a key constrains one relation's own rows"
)

bdb_compile_fail_case(schema_foreign_coordinate
    "references coordinate \"Outage.service\" but relation \"Outage\" is not a member of the schema"
)

bdb_compile_fail_case(query_var_cross_class_reuse
    "variable \"Outage.service\".*cannot bind coordinate \"Audit.actor\""
)

bdb_compile_fail_case(query_wrong_param_name
    "has no non-static data member named .at."
)

bdb_compile_fail_case(query_find_unbound_variable
    "find_head_variable_is_not_bound_in_this_rule"
)

bdb_compile_fail_case(closed_handle_wrong_vocabulary
    "handle \"Deterministic\" of closed relation \"Kind\" cannot bind coordinate \"Ticket.priority\""
)

bdb_compile_fail_case(discarded_expected
    "ignoring return value of .*ephemeral"
)

if(BDB_DARWIN)
    get_property(_bdb_tests DIRECTORY PROPERTY TESTS)
    set_tests_properties(${_bdb_tests} PROPERTIES ENVIRONMENT "MallocProbGuard=1")
endif()
