cmake_minimum_required(VERSION 4.0)

if(CMAKE_VERSION VERSION_LESS 4.2 OR CMAKE_VERSION VERSION_GREATER_EQUAL 4.3)
    message(FATAL_ERROR "CMake 4.2.x is required: the import-std experimental UUID below is specific to the 4.2 series (got ${CMAKE_VERSION})")
endif()

# The import-std experimental UUID is specific to the CMake 4.2 series; re-read Help/dev/experimental.rst on any bump.
set(CMAKE_EXPERIMENTAL_CXX_IMPORT_STD "d0edc3af-4c50-42ea-a356-e2862fe7a444")

project(bumbledb_cpp LANGUAGES CXX)

option(BDB_LINT_GRAPH
    "Configure the reflection-free Clang lint graph instead of the GCC production graph"
    OFF
)

# The configure step decides toolchain acceptability; translation units never negotiate features.
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

# Centralized import-std policy; never set per target.
set(CMAKE_CXX_MODULE_STD ON)

# Global, not profile-scoped: the std module BMI must share the exceptions/RTTI/reflection model with every importer, and CMake's synthesized import-std targets only see global flags.
string(APPEND CMAKE_CXX_FLAGS " -fno-exceptions -fno-rtti")

if(NOT BDB_LINT_GRAPH)
    string(APPEND CMAKE_CXX_FLAGS " -freflection")
endif()

# The language profile: the single build-system source of truth for the dialect.
add_library(bumbledb_language_profile INTERFACE)

target_compile_features(bumbledb_language_profile INTERFACE cxx_std_26)

if(NOT BDB_LINT_GRAPH)
    # stdc++exp (P2900 handle_contract_violation) as an interface library, never a linker flag: GNU ld resolves left-to-right and flags precede the objects.
    target_link_libraries(bumbledb_language_profile INTERFACE stdc++exp)
endif()

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

# src/ is GCC-only: the module's reflective partitions keep it out of the Clang lint graph.
if(NOT BDB_LINT_GRAPH)
    add_subdirectory(src)
endif()

# foreign/ belongs to both graphs and must come after src/: it adds :foreign_program to the bumbledb target in the production graph.
add_subdirectory(foreign)

enable_testing()
add_subdirectory(tests)

# The CMake-internal std-module target is not project code: never tidied, and compiled as strict c++26 or its BMI is rejected by our translation units.
if(TARGET __cmake_cxx26)
    set_property(TARGET __cmake_cxx26 PROPERTY CXX_EXTENSIONS OFF)
    if(BDB_LINT_GRAPH)
        set_property(TARGET __cmake_cxx26 PROPERTY CXX_CLANG_TIDY "")
    endif()
endif()
