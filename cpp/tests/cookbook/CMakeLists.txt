# The cookbook parity suite: each recipe asserts the engine's fingerprint readback
# against the shared golden (fixtures/cookbook-fingerprints.txt — the same file the
# TypeScript suite reads) and answers its recipe's own semantics against the real engine.
# PIN(gcc-template-for-wshadow): scoped -Wno-shadow on every reflective target.
# -Wno-missing-field-initializers: an unmentioned query/where-pattern slot IS the wildcard (lowering.md 4.1).

add_executable(bumbledb_cookbook_r01)

target_sources(bumbledb_cookbook_r01
    PRIVATE
        r01_uptime.cc
)

target_link_libraries(bumbledb_cookbook_r01
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r01 PRIVATE -Wno-shadow)

add_test(NAME cookbook.r01_uptime
    COMMAND bumbledb_cookbook_r01
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r01_queries)

target_sources(bumbledb_cookbook_r01_queries
    PRIVATE
        r01_queries.cc
)

target_link_libraries(bumbledb_cookbook_r01_queries
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r01_queries
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r01_queries COMMAND bumbledb_cookbook_r01_queries)

add_executable(bumbledb_cookbook_r02)

target_sources(bumbledb_cookbook_r02
    PRIVATE
        r02_grading.cc
)

target_link_libraries(bumbledb_cookbook_r02
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r02
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r02_grading
    COMMAND bumbledb_cookbook_r02
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r06)

target_sources(bumbledb_cookbook_r06
    PRIVATE
        r06_tickets.cc
)

target_link_libraries(bumbledb_cookbook_r06
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r06
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r06_tickets
    COMMAND bumbledb_cookbook_r06
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r07)

target_sources(bumbledb_cookbook_r07
    PRIVATE
        r07_review.cc
)

target_link_libraries(bumbledb_cookbook_r07
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r07
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r07_review
    COMMAND bumbledb_cookbook_r07
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r14)

target_sources(bumbledb_cookbook_r14
    PRIVATE
        r14_calendar.cc
)

target_link_libraries(bumbledb_cookbook_r14
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r14
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r14_calendar
    COMMAND bumbledb_cookbook_r14
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r24)

target_sources(bumbledb_cookbook_r24
    PRIVATE
        r24_closure.cc
)

target_link_libraries(bumbledb_cookbook_r24
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r24
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r24_closure
    COMMAND bumbledb_cookbook_r24
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r29)

target_sources(bumbledb_cookbook_r29
    PRIVATE
        r29_zone_ledger.cc
)

target_link_libraries(bumbledb_cookbook_r29
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r29
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r29_zone_ledger
    COMMAND bumbledb_cookbook_r29
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r31)

target_sources(bumbledb_cookbook_r31
    PRIVATE
        r31_power_budget.cc
)

target_link_libraries(bumbledb_cookbook_r31
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r31
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r31_power_budget
    COMMAND bumbledb_cookbook_r31
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r08)

target_sources(bumbledb_cookbook_r08
    PRIVATE
        r08_oncall.cc
)

target_link_libraries(bumbledb_cookbook_r08
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r08
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r08_oncall
    COMMAND bumbledb_cookbook_r08
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r19)

target_sources(bumbledb_cookbook_r19
    PRIVATE
        r19_ledger.cc
)

target_link_libraries(bumbledb_cookbook_r19
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r19
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r19_ledger
    COMMAND bumbledb_cookbook_r19
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r20)

target_sources(bumbledb_cookbook_r20
    PRIVATE
        r20_jobs.cc
)

target_link_libraries(bumbledb_cookbook_r20
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r20
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r20_jobs
    COMMAND bumbledb_cookbook_r20
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r21)

target_sources(bumbledb_cookbook_r21
    PRIVATE
        r21_rollup.cc
)

target_link_libraries(bumbledb_cookbook_r21
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r21
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r21_rollup
    COMMAND bumbledb_cookbook_r21
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r22)

target_sources(bumbledb_cookbook_r22
    PRIVATE
        r22_payments.cc
)

target_link_libraries(bumbledb_cookbook_r22
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r22
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r22_payments
    COMMAND bumbledb_cookbook_r22
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r23)

target_sources(bumbledb_cookbook_r23
    PRIVATE
        r23_gravestones.cc
)

target_link_libraries(bumbledb_cookbook_r23
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r23 PRIVATE -Wno-shadow)

add_test(NAME cookbook.r23_gravestones
    COMMAND bumbledb_cookbook_r23
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r28)

target_sources(bumbledb_cookbook_r28
    PRIVATE
        r28_migration.cc
)

target_link_libraries(bumbledb_cookbook_r28
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r28
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r28_migration
    COMMAND bumbledb_cookbook_r28
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r30)

target_sources(bumbledb_cookbook_r30
    PRIVATE
        r30_keyed_read.cc
)

target_link_libraries(bumbledb_cookbook_r30
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r30
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r30_keyed_read
    COMMAND bumbledb_cookbook_r30
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r32)

target_sources(bumbledb_cookbook_r32
    PRIVATE
        r32_calendar_capacity.cc
)

target_link_libraries(bumbledb_cookbook_r32
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r32
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r32_calendar_capacity
    COMMAND bumbledb_cookbook_r32
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r03)

target_sources(bumbledb_cookbook_r03
    PRIVATE
        r03_optionality.cc
)

target_link_libraries(bumbledb_cookbook_r03
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r03
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r03_optionality
    COMMAND bumbledb_cookbook_r03
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r04)

target_sources(bumbledb_cookbook_r04
    PRIVATE
        r04_money.cc
)

target_link_libraries(bumbledb_cookbook_r04
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r04
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r04_money
    COMMAND bumbledb_cookbook_r04
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r05)

target_sources(bumbledb_cookbook_r05
    PRIVATE
        r05_content.cc
)

target_link_libraries(bumbledb_cookbook_r05
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r05
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r05_content
    COMMAND bumbledb_cookbook_r05
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r25)

target_sources(bumbledb_cookbook_r25
    PRIVATE
        r25_accounts.cc
)

target_link_libraries(bumbledb_cookbook_r25
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r25
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r25_accounts
    COMMAND bumbledb_cookbook_r25
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r26)

target_sources(bumbledb_cookbook_r26
    PRIVATE
        r26_exact_partition.cc
)

target_link_libraries(bumbledb_cookbook_r26
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r26
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r26_exact_partition
    COMMAND bumbledb_cookbook_r26
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r27)

target_sources(bumbledb_cookbook_r27
    PRIVATE
        r27_maintained_rollup.cc
)

target_link_libraries(bumbledb_cookbook_r27
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r27
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r27_maintained_rollup
    COMMAND bumbledb_cookbook_r27
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r09)

target_sources(bumbledb_cookbook_r09
    PRIVATE
        r09_playlists.cc
)

target_link_libraries(bumbledb_cookbook_r09
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r09
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r09_playlists
    COMMAND bumbledb_cookbook_r09
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r10)

target_sources(bumbledb_cookbook_r10
    PRIVATE
        r10_ast.cc
)

target_link_libraries(bumbledb_cookbook_r10
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r10
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r10_ast
    COMMAND bumbledb_cookbook_r10
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r11)

target_sources(bumbledb_cookbook_r11
    PRIVATE
        r11_graph.cc
)

target_link_libraries(bumbledb_cookbook_r11
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r11
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r11_graph
    COMMAND bumbledb_cookbook_r11
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r12)

target_sources(bumbledb_cookbook_r12
    PRIVATE
        r12_ecs.cc
)

target_link_libraries(bumbledb_cookbook_r12
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r12
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r12_ecs
    COMMAND bumbledb_cookbook_r12
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r13)

target_sources(bumbledb_cookbook_r13
    PRIVATE
        r13_orders.cc
)

target_link_libraries(bumbledb_cookbook_r13
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r13
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r13_orders
    COMMAND bumbledb_cookbook_r13
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r15)

target_sources(bumbledb_cookbook_r15
    PRIVATE
        r15_pricing.cc
)

target_link_libraries(bumbledb_cookbook_r15
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r15
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r15_pricing
    COMMAND bumbledb_cookbook_r15
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r16)

target_sources(bumbledb_cookbook_r16
    PRIVATE
        r16_payroll.cc
)

target_link_libraries(bumbledb_cookbook_r16
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r16
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r16_payroll
    COMMAND bumbledb_cookbook_r16
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r17)

target_sources(bumbledb_cookbook_r17
    PRIVATE
        r17_tax.cc
)

target_link_libraries(bumbledb_cookbook_r17
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r17
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r17_tax
    COMMAND bumbledb_cookbook_r17
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

add_executable(bumbledb_cookbook_r18)

target_sources(bumbledb_cookbook_r18
    PRIVATE
        r18_freetime.cc
)

target_link_libraries(bumbledb_cookbook_r18
    PRIVATE
        bumbledb
        bumbledb_language_profile
)

target_compile_options(bumbledb_cookbook_r18
    PRIVATE -Wno-shadow -Wno-missing-field-initializers)

add_test(NAME cookbook.r18_freetime
    COMMAND bumbledb_cookbook_r18
        ${CMAKE_SOURCE_DIR}/../fixtures/cookbook-fingerprints.txt
)

if(BDB_DARWIN)
    get_property(_bdb_tests DIRECTORY PROPERTY TESTS)
    set_tests_properties(${_bdb_tests} PROPERTIES ENVIRONMENT "MallocProbGuard=1")
endif()
