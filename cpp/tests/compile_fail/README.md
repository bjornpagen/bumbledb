# tests/compile_fail/

The expected-failure compiler harness (TODO_CPP §34): the SDK is not
complete if only successful programs compile, so each case here is one
translation unit that MUST fail to compile AND MUST emit a pinned
diagnostic substring.

Mechanism (see CMakeLists.txt): each case is an `EXCLUDE_FROM_ALL` object
library `cf_<case>` — a real project target, so cases can import project
modules — and a CTest test `compile_fail.<case>` that invokes
`cmake --build ... --target cf_<case>` with
`PASS_REGULAR_EXPRESSION "<pinned diagnostic substring>"`. The test passes
only when the build fails with the pinned diagnostic:

- a regression that silently starts ACCEPTING the invalid program produces
  no diagnostic, the regex does not match, and the test fails CI;
- a diagnostic that degrades into template noise stops matching its pinned
  substring and likewise fails.

Diagnostics are part of the SDK product: pin semantic coordinates
(`Outage.service`, `Repo.id`) as the substrings once the elaborator exists,
never template internals. The two seed cases prove the harness itself: a
`static_assert` with a distinctive message, and an import of a module no
rule provides.
