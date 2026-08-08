add_library(bumbledb_version)

target_sources(bumbledb_version
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            bumbledb.version.cppm
    PRIVATE
        bumbledb.version.cpp
)

target_link_libraries(bumbledb_version
    PUBLIC
        bumbledb_language_profile
)
