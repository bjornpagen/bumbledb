# One importable named module per component: `bumbledb` is the SDK's only
# export surface and its internals are partitions, so outsiders physically
# cannot import them. Every partition file plus the primary interface lives
# in this one FILE_SET; CMake scans the partition dependencies itself.
#
# GCC-only: the module contains reflective partitions (the relation
# reflector, the schema elaborator, the query builder), so the whole module
# is outside the Clang lint graph — the top-level CMakeLists adds this
# directory only in the production graph. The :foreign_program partition is
# quarantine code and is added to this target from foreign/CMakeLists.txt
# (it lives in foreign/ but belongs to THIS module — see that file).
add_library(bumbledb)

target_sources(bumbledb
    PUBLIC
        FILE_SET CXX_MODULES
        BASE_DIRS ${PROJECT_SOURCE_DIR}
        FILES
            bumbledb.cc
            error.cc
            order.cc
            version.cc
            types/interval.cc
            types/bytes.cc
            types/allen.cc
            types/fresh.cc
            relation/name.cc
            relation/classify.cc
            relation/coord.cc
            relation/facade.cc
            relation/row.cc
            closed/handle.cc
            closed/id.cc
            closed/member.cc
            closed/axioms.cc
            closed/facade.cc
            closed/where.cc
            schema/spec.cc
            schema/member.cc
            schema/face.cc
            schema/key.cc
            schema/contained.cc
            schema/mirrors.cc
            schema/capacity.cc
            schema/unionfind.cc
            schema/classes.cc
            schema/schema.cc
            query/ir.cc
            query/var.cc
            query/param.cc
            query/pattern.cc
            query/predicate.cc
            query/aggregate.cc
            query/head.cc
            query/rule.cc
            query/lower.cc
            query/query.cc
            query/program.cc
            answers/decode.cc
            answers/answers.cc
            answers/row.cc
            db/manifest.cc
            db/wire.cc
            db/write.cc
            db/prepared.cc
            db/snapshot.cc
            db/tx.cc
            db/db.cc
    PRIVATE
        # Module implementation unit (no BMI): holds the Db member bodies a
        # pinned GCC 16.1 quirk keeps out of the :db partition — see the
        # comment in db/db_impl.cc.
        db/db_impl.cc
)

# Pinned GCC 16.1 quirk (TODO_CPP §32): expansion statements (`template
# for`) trip -Wshadow on compiler-generated scoping whenever the expanded
# range has more than one element. The reflective partitions instantiate
# expansion statements, so the module target carries a scoped -Wno-shadow —
# never an inline suppression. The same option sits on every TU that
# INSTANTIATES the reflective templates (runtime meta tests, compile-fail
# cases): expansion happens at the instantiation site.
target_compile_options(bumbledb PRIVATE -Wno-shadow)

target_link_libraries(bumbledb
    PUBLIC
        bumbledb_language_profile
        bumbledb_foreign
)
