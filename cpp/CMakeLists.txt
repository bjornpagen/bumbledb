cmake_minimum_required(VERSION 4.0)

if(NOT CMAKE_GENERATOR STREQUAL "Ninja")
    message(FATAL_ERROR "Ninja is the only supported generator (got ${CMAKE_GENERATOR})")
endif()

if(NOT CMAKE_MAKE_PROGRAM)
    find_program(CMAKE_MAKE_PROGRAM ninja REQUIRED)
endif()

execute_process(
    COMMAND "${CMAKE_MAKE_PROGRAM}" --version
    RESULT_VARIABLE BDB_NINJA_STATUS
    OUTPUT_VARIABLE BDB_NINJA_VERSION
    OUTPUT_STRIP_TRAILING_WHITESPACE
)
if(NOT BDB_NINJA_STATUS EQUAL 0 OR NOT BDB_NINJA_VERSION MATCHES "^1\\.13\\.")
    message(FATAL_ERROR "Ninja 1.13.x is required (got ${BDB_NINJA_VERSION})")
endif()

if(CMAKE_VERSION VERSION_LESS 4.2 OR CMAKE_VERSION VERSION_GREATER_EQUAL 4.3)
    message(FATAL_ERROR "CMake 4.2.x is required: the import-std experimental UUID below is specific to the 4.2 series (got ${CMAKE_VERSION})")
endif()

# PIN(cmake-import-std-uuid): the 4.2-series value — see PINS.md
set(CMAKE_EXPERIMENTAL_CXX_IMPORT_STD "d0edc3af-4c50-42ea-a356-e2862fe7a444")

project(bumbledb_cpp LANGUAGES CXX)

if(CMAKE_SYSTEM_NAME STREQUAL "Darwin")
    set(BDB_DARWIN TRUE)
    set(BDB_LINUX FALSE)
elseif(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    set(BDB_DARWIN FALSE)
    set(BDB_LINUX TRUE)
else()
    set(BDB_DARWIN FALSE)
    set(BDB_LINUX FALSE)
endif()

option(BDB_LINT_GRAPH
    "Configure the reflection-free Clang lint graph instead of the GCC production graph"
    OFF
)
option(BDB_RUNTIME_SANITIZERS
    "Use runtime ASan+UBSan instead of trap-mode UBSan (the asan-ubsan preset)"
    OFF
)

if(BDB_LINT_GRAPH)
    if(NOT CMAKE_CXX_COMPILER_ID STREQUAL "Clang")
        message(FATAL_ERROR "the lint graph requires the pinned Clang toolchain (got ${CMAKE_CXX_COMPILER_ID})")
    endif()
    if(CMAKE_CXX_COMPILER_VERSION VERSION_LESS 22.1
            OR CMAKE_CXX_COMPILER_VERSION VERSION_GREATER_EQUAL 22.2)
        message(FATAL_ERROR "the lint graph requires the pinned Clang 22.1.x (got ${CMAKE_CXX_COMPILER_VERSION})")
    endif()
    if(NOT CMAKE_CXX_CLANG_TIDY)
        message(FATAL_ERROR "the lint graph requires the pinned clang-tidy")
    endif()
    list(GET CMAKE_CXX_CLANG_TIDY 0 BDB_CLANG_TIDY)
    execute_process(
        COMMAND "${BDB_CLANG_TIDY}" --version
        RESULT_VARIABLE BDB_CLANG_TIDY_STATUS
        OUTPUT_VARIABLE BDB_CLANG_TIDY_VERSION
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )
    if(NOT BDB_CLANG_TIDY_STATUS EQUAL 0 OR NOT BDB_CLANG_TIDY_VERSION MATCHES "LLVM version 22\\.1\\.")
        message(FATAL_ERROR "the lint graph requires clang-tidy 22.1.x (got ${BDB_CLANG_TIDY_VERSION})")
    endif()
else()
    if(NOT CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
        message(FATAL_ERROR "production builds require the pinned GCC toolchain (got ${CMAKE_CXX_COMPILER_ID})")
    endif()
    if(CMAKE_CXX_COMPILER_VERSION VERSION_LESS 16.1)
        message(FATAL_ERROR "production builds require GCC 16.1 or later (got ${CMAKE_CXX_COMPILER_VERSION})")
    endif()
endif()

set(CMAKE_CXX_STANDARD 26)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_CXX_MODULE_STD ON)

# Module-ABI flags must be global: CMake's synthesized import-std targets see
# only CMAKE_CXX_FLAGS, and the std module BMI must match every importer.
# PIN(clang-module-stack-protector): Clang 22.1 records these as BMI config bits
string(APPEND CMAKE_CXX_FLAGS " -fno-exceptions -fno-rtti -D_GLIBCXX_ASSERTIONS -ftrivial-auto-var-init=zero -fstack-protector-strong -fzero-call-used-regs=used-gpr")

if(NOT BDB_LINT_GRAPH)
    # PIN(cmake-ipo-probe-ordering): IPO probe must precede -freflection
    # PIN(gcc-darwin-lto-debug-dsymutil): Release-only IPO, never -flto -g
    include(CheckIPOSupported)
    check_ipo_supported(LANGUAGES CXX)
    set(CMAKE_INTERPROCEDURAL_OPTIMIZATION_RELEASE ON)
    string(APPEND CMAKE_CXX_FLAGS " -freflection")
endif()

add_library(bumbledb_language_profile INTERFACE)

if(NOT BDB_LINT_GRAPH)
    # PIN(cmake-ld-link-order): contracts runtime after every consumer — see PINS.md
    target_link_libraries(bumbledb_language_profile INTERFACE stdc++exp)
    if(NOT BDB_RUNTIME_SANITIZERS)
        target_compile_options(bumbledb_language_profile INTERFACE
            -fsanitize=undefined
            -fsanitize-trap=all
            "$<$<CONFIG:Release>:-ffunction-sections;-fdata-sections>"
        )
        target_link_options(bumbledb_language_profile INTERFACE
            -fsanitize=undefined
            -fsanitize-trap=all
            "$<$<AND:$<CONFIG:Release>,$<BOOL:${BDB_DARWIN}>>:LINKER:-dead_strip>"
            "$<$<AND:$<CONFIG:Release>,$<BOOL:${BDB_LINUX}>>:LINKER:--gc-sections>"
        )
    endif()
endif()

target_compile_options(bumbledb_language_profile INTERFACE
    "$<$<CXX_COMPILER_ID:GNU>:-fstack-clash-protection;-fno-delete-null-pointer-checks;-fno-strict-aliasing;-fstrict-flex-arrays=3;-fzero-init-padding-bits=all>"
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
    -Wformat
    -Wformat=2
    -Werror=format-security
    -Wimplicit-fallthrough
    "$<$<CXX_COMPILER_ID:GNU>:-Wuseless-cast;-Wduplicated-branches;-Wduplicated-cond;-Wlogical-op;-Wtrampolines;-Wbidi-chars=any>"
)

# PIN(darwin-inert-mitigations): FORTIFY and PAC/BTI are Linux-only — see PINS.md
if(BDB_LINUX)
    target_compile_options(bumbledb_language_profile INTERFACE
        -fPIE
        "$<$<CXX_COMPILER_ID:GNU>:-mbranch-protection=standard>"
    )
    target_link_options(bumbledb_language_profile INTERFACE
        -pie
        LINKER:-z,relro
        LINKER:-z,now
        LINKER:-z,noexecstack
        LINKER:-z,nodlopen
        LINKER:--as-needed
        LINKER:--no-copy-dt-needed-entries
    )
    if(NOT BDB_RUNTIME_SANITIZERS)
        target_compile_options(bumbledb_language_profile INTERFACE
            -U_FORTIFY_SOURCE
            -D_FORTIFY_SOURCE=3
        )
    endif()
endif()

# src/ is GCC-only: the module's reflective partitions keep it out of the Clang lint graph.
if(NOT BDB_LINT_GRAPH)
    add_subdirectory(src)
endif()

# foreign/ belongs to both graphs and must come after src/: it adds :foreign_program to the bumbledb target in the production graph.
add_subdirectory(foreign)

enable_testing()
if(NOT BDB_LINT_GRAPH)
    add_subdirectory(tests)
endif()

# The CMake-internal std-module target is not project code: never tidied, and compiled as strict c++26 or its BMI is rejected by our translation units.
if(TARGET __cmake_cxx26)
    set_property(TARGET __cmake_cxx26 PROPERTY CXX_EXTENSIONS OFF)
    if(BDB_LINT_GRAPH)
        set_property(TARGET __cmake_cxx26 PROPERTY CXX_CLANG_TIDY "")
    endif()
endif()
