# The one SDK module: internals are partitions, so outsiders physically cannot import them; CMake scans partition dependencies from the one FILE_SET.
# The :foreign_program partition is added to this target from foreign/CMakeLists.txt.
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
        # PIN(gcc-partition-bmi-expected): module implementation unit, no BMI
        db/db_impl.cc
)

# PIN(gcc-template-for-wshadow)
target_compile_options(bumbledb PRIVATE -Wno-shadow)

target_link_libraries(bumbledb
    PUBLIC
        bumbledb_language_profile
        bumbledb_foreign
)
