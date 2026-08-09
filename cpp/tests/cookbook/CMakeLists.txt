# The cookbook parity suite (TODO_CPP §33): each recipe builds its theory
# through the real bdb::schema<> elaborator, creates a store, and asserts
# the engine's fingerprint readback equals the host-neutral golden at the
# repository root — the SAME fixtures file the TypeScript suite reads
# (fixtures/cookbook-fingerprints.txt), passed as a test argument.
#
# GCC-only (the schema elaborator is a reflective partition of `bumbledb`) — this
# directory is added only in the production graph.

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

# Recipe 1's QUERIES against the real engine (TODO_CPP §11-§12, §20-§23):
# prepare through the engine validator, execute, assert the recipe's own
# answers. -Wno-missing-field-initializers: an unmentioned query-pattern
# slot IS the wildcard (lowering.md §4.1), so query-writing TUs suppress
# exactly this -Wextra lint, the scoped way (-Wno-shadow's precedent).
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

# Recipe 2 — Discriminated unions (Phase F): the closed discriminator, the
# ψ-selected mirrors, and the host handle-projection dispatch. Fingerprint
# vs the shared golden. -Wno-missing-field-initializers: an unmentioned
# where-pattern slot binds nothing, same suppression story as the query
# patterns.
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

# Recipe 6 — The vocabulary (Phase F): the bare-tier closed relation, the
# handle-literal query, and the membership-array ∈-set query, run against
# the real engine.
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

# Recipe 7 — The classification (Phase F): the payload-tier closed
# vocabulary, the ψ-selected containment target, the closed atom with a
# bool payload literal, and the typed axiom readback.
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

# Recipe 8 — The sub-vocabulary (Phase F): the ψ-selected containment
# target as the compiled member set — nonmember writes commit-rejected —
# and the same ψ on the read side.
# Recipe 14 — The calendar core (Phase G): ψ-selected mirrors on BOTH
# sides, the ψ-selected multi-column coverage containment, Allen-vs-param
# queries answered live, and the pack coalescing fold (recipe 18's shape)
# against the real engine.
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

# Recipe 24 — The closure idiom (Phase G): the runtime ∈-set param, the
# stratified program (bdb::program / bdb::rec / .idb), negation of the
# finished stratum (.not_idb), and the host-loop/native-program agreement.
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

# Recipe 29 — The zone ledger (Phase G): the fixed-width interval family
# (interval<u64, 1> / <u64, 2>) as TYPE-enforced arm widths, the
# kind-discriminated witness with per-kind ψ mirrors at mixed widths, and
# the coalescing-insensitivity write.
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

# Recipe 31 — The power budget (Phase G): the weighted capacity law with
# the pinned-column idiom, sum over a plain scalar variable, the
# count/max/argMax head ops, the over-budget capacity citation, and the
# witnessed write loop.
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

# writes — recipes 19-23 (ts/COOKBOOK.md "The write side").

# Recipe 19 — The ledger: balance is a query, never a column; both
# rollup reads fold sum over a plain scalar variable, fresh id bound.
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

# Recipe 20 — Conditional writes: the witnessed update-where RUNS —
# premise via snap, delta via tx, abandonment-as-data on the empty store.
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

# Recipe 21 — Derived relations: the ψ-selected multi-column soundness
# containment, the deriving pack query. Shares recipe 27's hex by design.
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

# Recipe 22 — Union reads: one query, two rules, same head {id, n} —
# bdb::as decouples the head name from the per-arm field.
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

# Recipe 23 — The anti-recipes: the five gravestones' REPLACEMENTS,
# compiled; the refused spellings have no C++ spelling to begin with.
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

# ops — Recipes 28/30/32 (Operating the store / point reads / capacity).
#
# Recipe 28 — Migration is ETL: two theories, two fingerprints (the
# refusal's premise), the v2 target pinned to the shared golden, and the
# post-migration `inForceAt` read prepared and answered against the real
# engine.
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

# Recipe 30 — The keyed read: the stored key law as the get() selector on
# every scope (db.get / snap.get / tx.get final-state), plus the
# fresh-field primary lane.
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

# Recipe 32 — Calendar capacity: the Duration weight against the Duration
# bound, the pointwise key beside the capacity law (double-booking vs
# over-total rejections cite different statements), and the
# sum-of-durations read.
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

# foundations
# Recipes 3-5 — the Foundations chapter's remaining trio:
#   r03 Optionality — 0..1 as a keyed side relation with a one-way
#       containment; the anti-join query (one negated EDB atom).
#   r04 Money — minor units as bare i64, the currency as a closed
#       vocabulary, the balance as a grouped scalar-sum query.
#   r05 Content — bytes(32) digests: keyed content addressing, the
#       generator-less bytes -> bytes containment, and the bytes param
#       bound in the match record.
# Each asserts its fingerprint against the shared golden and answers its
# recipe's own semantics against the real engine.
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

# closure — recipes 25/26/27 (host-driven closure block, r24's sibling
# recipes; r24's own entry lives above).
#
# Recipe 25 — The chart of accounts: the host composition (recipe 24's
# frontier loop, then ONE ∈-set + sum fold), the engine-native program
# whose output folds over the finished lower stratum, and the two
# dialects' agreement, subtree for subtree.
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

# Recipe 26 — Exact partition: mutual multi-column coverage plus the two
# pointwise keys realize exact partition — the tiled write commits, a
# gap and an overhang are commit-rejected.
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

# Recipe 27 — Derived facts, maintained: the stored rollup as an ordinary
# relation, the ψ-selected soundness containment, and the deriving pack
# query (the r21 hex — the theories coincide, by design).
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

# structure — recipes 9-13 (ts/COOKBOOK.md "Structure"): ordered
# collections (the interval-partition triple, the mixed-width tiling
# mirrors, the shipped host comparator), trees/ASTs (per-kind arms +
# resolving edges), typed graphs (endpoint containments + the var-var lt
# self-join), entity-component (the 0..1 idiom at scale + the archetype
# containment), and state machines (the ψ-selected conditional reference
# target). Each asserts its fixtures golden and answers its recipe's own
# semantics against the real engine.
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

# time — recipes 15-18 (Time and coverage).
# Recipe 15 — Effective-dated configuration: the disjoint cover (pointwise
# key + one-way coverage, overhang legal), the in-force membership probe,
# and the successions VAR-VAR allen(a, meets, b) self-join.
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

# Recipe 16 — Disjoint covers: sequence-number and pointwise keys plus the
# one-way coverage containment; the holding(t) membership probe.
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

# Recipe 17 — Federal income tax: the (year, status) key over a
# closed-handle column, ray brackets ([s, MAX) as an honest value), the
# residency-exclusion coverage, and the marginal query binding a param AT
# the closed-handle field.
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

# Recipe 18 — Free time and coalescing: the deliberately keyless claims,
# the pack coalescing fold, and the raw sum(duration) twin.
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
