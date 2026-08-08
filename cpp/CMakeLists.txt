cmake_minimum_required(VERSION 4.0)

if(CMAKE_VERSION VERSION_LESS 4.2 OR CMAKE_VERSION VERSION_GREATER_EQUAL 4.3)
    message(FATAL_ERROR "CMake 4.2.x is required: the import-std experimental UUID below is specific to the 4.2 series (got ${CMAKE_VERSION})")
endif()

# `import std` is experimental in CMake. This UUID gates the feature and is
# specific to the CMake 4.2 series. On any CMake bump, re-read
# Help/dev/experimental.rst and update the value.
set(CMAKE_EXPERIMENTAL_CXX_IMPORT_STD "d0edc3af-4c50-42ea-a356-e2862fe7a444")

project(bumbledb_cpp LANGUAGES CXX)

option(BDB_LINT_GRAPH
    "Configure the reflection-free Clang lint graph instead of the GCC production graph"
    OFF
)

# The configure step decides whether the toolchain is acceptable; translation
# units never negotiate features. Production: GCC 16.1.x. Lint: Clang 22.x.
# Acquiring the toolchain is the developer's business (see AGENTS.md).
if(BDB_LINT_GRAPH)
    if(NOT CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
        message(FATAL_ERROR "the lint graph requires the pinned Clang toolchain (got ${CMAKE_CXX_COMPILER_ID})")
    endif()
    if(CMAKE_CXX_COMPILER_VERSION VERSION_LESS 22
            OR CMAKE_CXX_COMPILER_VERSION VERSION_GREATER_EQUAL 23)
        message(FATAL_ERROR "the lint graph requires the pinned Clang 22.x (got ${CMAKE_CXX_COMPILER_VERSION})")
    endif()
else()
    if(NOT CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
        message(FATAL_ERROR "production builds require the pinned GCC toolchain (got ${CMAKE_CXX_COMPILER_ID})")
    endif()
    if(CMAKE_CXX_COMPILER_VERSION VERSION_LESS 16.1
            OR CMAKE_CXX_COMPILER_VERSION VERSION_GREATER_EQUAL 16.2)
        message(FATAL_ERROR "production builds require the pinned GCC 16.1.x (got ${CMAKE_CXX_COMPILER_VERSION})")
    endif()
endif()

set(CMAKE_CXX_STANDARD 26)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Centralized `import std` policy: every dialect target consumes the
# standard-library named module. Do not set this per target.
set(CMAKE_CXX_MODULE_STD ON)

# Exceptions/RTTI/reflection are part of the module ABI: the std module BMI
# must be built with the same model as every importer (and `-freflection` is
# required for the std module to export std::meta at all). CMake's
# synthesized import-std targets only see global flags — so these are
# global, not part of the language-profile target.
string(APPEND CMAKE_CXX_FLAGS " -fno-exceptions -fno-rtti")

if(NOT BDB_LINT_GRAPH)
    # Reflection is not negotiable: the version gate above guarantees GCC
    # 16.1.x, which implements it.
    string(APPEND CMAKE_CXX_FLAGS " -freflection")
    # Contracts (P2900): handle_contract_violation links from libstdc++exp.
    string(APPEND CMAKE_EXE_LINKER_FLAGS " -lstdc++exp")
endif()

# ----------------------------------------------------------------------------
# Language profile: the single build-system source of truth for the dialect.
# Every dialect target links this and nothing redefines these requirements.
# ----------------------------------------------------------------------------
add_library(bumbledb_language_profile INTERFACE)

target_compile_features(bumbledb_language_profile INTERFACE cxx_std_26)

# -fno-exceptions / -fno-rtti are global (see CMAKE_CXX_FLAGS above): they
# must also apply to the CMake-synthesized std module compilation.
target_compile_options(bumbledb_language_profile INTERFACE
    -Werror
    -Wall
    -Wextra
    -Wpedantic
    -Wconversion
    -Wsign-conversion
    -Wshadow
    -Wold-style-cast
    -Wcast-align
    -Wnull-dereference
    -Wdouble-promotion
    "$<$<CXX_COMPILER_ID:GNU>:-Wuseless-cast;-Wduplicated-branches;-Wduplicated-cond;-Wlogical-op>"
)

add_subdirectory(src)

# meta/ holds GCC-only reflection modules; it is never part of the lint graph.
if(NOT BDB_LINT_GRAPH)
    add_subdirectory(meta)
endif()

# foreign/ is the quarantine zone: the cargo-built Rust bridge staticlib and
# the one wrapper module allowed to see the generated C ABI header. It holds
# no reflection, so it is part of both graphs.
add_subdirectory(foreign)

enable_testing()
add_subdirectory(tests)

# The CMake-internal target that builds the standard-library module is not
# project code: clang-tidy must not run over it, and it must be compiled as
# strict c++26 (not gnu++26) or its BMI is rejected by our translation units.
if(TARGET __cmake_cxx26)
    set_property(TARGET __cmake_cxx26 PROPERTY CXX_EXTENSIONS OFF)
    if(BDB_LINT_GRAPH)
        set_property(TARGET __cmake_cxx26 PROPERTY CXX_CLANG_TIDY "")
    endif()
endif()
