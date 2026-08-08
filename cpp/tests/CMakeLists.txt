add_subdirectory(runtime)

# The compile-fail harness drives the production compiler and pins GCC
# diagnostic text (TODO_CPP §32, §34); it is not part of the lint graph.
if(NOT BDB_LINT_GRAPH)
    add_subdirectory(compile_fail)
endif()

# cookbook/ and bridge/ are README-only until their phases land
# (TODO_CPP §33, §35).
