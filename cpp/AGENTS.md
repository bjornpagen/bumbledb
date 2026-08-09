# AGENTS.md

## Purpose

This repository uses a deliberately small C++26 language profile.

The goal is not “idiomatic C++” in the broad ecosystem sense. The goal is a
single, coherent systems language built from the newest static C++ mechanisms:

- values and algebraic data types,
- structural concepts,
- templates and constraints,
- C++26 reflection and annotations,
- `constexpr` / `consteval`,
- RAII resource capabilities,
- `std::expected`,
- structured sender/receiver execution,
- named modules,
- explicit unsafe/foreign boundaries.

When C++ offers multiple mechanisms for the same concept, this project chooses
one. Older, weaker, dynamic, textual, or nominal substitutes are forbidden.

This document is normative for humans and coding agents.

---

## 1. Prime directive: one concept, one primitive

When two mechanisms overlap, use the blessed mechanism below and reject the
alternative.

| Concept | Blessed mechanism | Forbidden alternatives |
|---|---|---|
| Generic polymorphism | concepts + templates | inheritance, virtual interfaces, CRTP, tag dispatch |
| Closed polymorphism | `std::variant` + visitation | class hierarchies, manual tagged unions |
| Structural introspection | C++26 reflection | RTTI, registries, X-macros, duplicated field lists |
| Compile-time elaboration | `consteval` + reflection | preprocessor metaprogramming, generated boilerplate when reflection can derive it |
| Synchronous failure | `std::expected<T, E>` | exceptions, error-code + out-param APIs |
| Optionality | `std::optional<T>` | null sentinels, magic values |
| Async effects | `std::execution` sender/receiver | `std::async`, raw futures/promises, direct coroutines, detached callbacks |
| Parallel composition | sender combinators such as `when_all` | manually coordinated threads |
| Resource lifetime | RAII | manual cleanup paths |
| Dynamic ownership | `std::unique_ptr<T>` when a value cannot suffice | raw owning pointers, `shared_ptr`, `weak_ptr` |
| Required borrow | `T&` / `T const&` | raw pointers |
| Optional borrow | `std::optional<T&>` | nullable raw pointers, `std::optional<std::reference_wrapper<T>>` |
| Sequence borrow | `std::span<T>` | pointer + length |
| Text borrow | `std::string_view` | pointer + length / null-terminated borrowed APIs |
| Finite atoms | `enum class` | integer constants, strings-as-enums |
| Product data | public `struct` | getter/setter object shells |
| Stateful capability/resource | small RAII `class` | “manager” object graphs |
| Project dependencies | named modules | project headers, header units, textual inclusion |
| Formatting/output | `std::format` / `std::print` | iostream formatting APIs, printf-family APIs |
| Configuration | build graph / typed values | project preprocessor conditionals |
| Semantic compatibility | structural concepts and compile-time laws | nominal tag wrappers / marker base classes |

If the blessed mechanism cannot express a required property, stop and identify
the missing primitive before introducing a second way.

---

## 2. Toolchain

Production code targets exactly one pinned toolchain tuple: the production
compiler + its matching standard library, the build system, the generator, and
the separately pinned clang-tidy build for the reflection-free lint graph.

The exact versions are enforced by the top-level CMake configure gate. They
live there and only there; documentation must not duplicate version numbers.
Toolchain versions are part of the language implementation, not ambient
developer-machine state.

Required project compiler properties:

- C++26 mode,
- reflection enabled,
- modules enabled,
- exceptions disabled,
- RTTI disabled.

The build must use the equivalent of:

```text
-std=c++26
-freflection
-fno-exceptions
-fno-rtti
```

`-fno-threadsafe-statics` is deliberately NOT used: on conforming code it is
a no-op (the statics it affects are banned outright, §14), so its only
observable effect would be converting a ban violation — or a vendored
dependency's internal static in `foreign/` — into an initialization race
under threads. A flag whose only behavior is making violations more
dangerous has negative value.

Warnings are errors. Use the strongest practical conversion, lifetime,
undefined-behavior, and API diagnostics for the pinned compiler.

Do not add compatibility branches for older standards, older GCC versions,
Clang, MSVC, or alternative standard libraries inside dialect code.

The configure/build step decides whether the pinned toolchain is acceptable.
Translation units do not perform feature negotiation.

---

## 3. Build system: CMake describes, Ninja executes

The only project build system is **CMake generating Ninja**.

Do not introduce Makefiles, Meson, Xmake, Bazel, Premake, hand-written Ninja,
or a second project build graph.

The architectural split is:

```text
CMake
  -> declares targets, modules, usage requirements, configurations, and tests
Ninja
  -> executes the generated dependency graph
GCC
  -> authoritative production compiler
Clang
  -> secondary reflection-free lint frontend
```

### 3.1 CMake is declarative project metadata

CMake must remain boring.

Use CMake for:

- declaring targets,
- explicitly listing the primary interface and every partition file,
- target-scoped compile features/options,
- target-scoped dependencies,
- selecting one platform/foreign implementation,
- defining tests,
- defining build configurations/presets.

Do **not** use CMake as a general-purpose programming language.

Forbidden unless there is a documented build-system-level necessity:

- project-defined CMake `macro()` abstractions,
- project-defined CMake `function()` abstraction layers,
- source-tree code generation that C++26 reflection could replace,
- `file(GLOB ...)` / `GLOB_RECURSE` for project sources,
- `execute_process()` as application logic,
- shell pipelines embedded in target definitions,
- compiler-feature negotiation scattered across targets,
- dynamically mutating the build graph based on incidental host state.

A small top-level platform/toolchain conditional is acceptable when it selects
one explicitly supported foreign implementation. Do not reproduce `#ifdef`
architecture in CMake.

### 3.2 Module declarations are explicit

Each component is one CMake module library target: the primary interface and
every partition file are explicitly listed in that one target's
`FILE_SET CXX_MODULES`.

Do not glob module files.

Conceptually:

```cmake
add_library(project_module)

target_sources(project_module
    PUBLIC
        FILE_SET CXX_MODULES
        FILES
            project.cc
            core.cc
            schema.cc
            query.cc
)
```

CMake owns module dependency scanning and BMI scheduling — including the
partition-import edges between the files above. Never hand-construct BMI
dependency edges or compiler module maps in project code.

Targets that use the standard-library named module enable `CXX_MODULE_STD` in
one centralized project helper/target policy rather than setting it ad hoc.

The CMake experimental gate required by the pinned CMake release for
`import std;` is configured once in the top-level toolchain/preset. Do not
scatter it across targets.

### 3.3 Ninja is the only generator

Use the Ninja generator for every supported build preset.

Do not hand-edit `build.ninja`.
Do not check generated Ninja files into source control.
Do not make direct Ninja commands the documented developer interface.

The human/agent interface is CMake Presets; Ninja is an implementation detail.

### 3.4 Presets are the canonical developer interface

`CMakePresets.json` is normative.

Provide exactly the supported build personalities, such as:

```text
dev
release
asan-ubsan
tsan
lint
```

Each preset has its own build directory.

Do not ask developers or agents to remember long ad-hoc CMake command lines.

The production presets use GCC.
The lint preset uses the separately pinned Clang toolchain and includes only
the reflection-free lint graph.

Enable compilation database generation for configurations consumed by tooling.

### 3.5 One language-profile target

Compiler and language-profile requirements must be centralized in one CMake
interface target (for example `project_language_profile`) and linked by every
dialect target.

That target is the single build-system source of truth for requirements such as:

```text
C++26
-freflection             GCC production graph only
-fno-exceptions
-fno-rtti
warnings-as-errors
conversion warnings
project-wide diagnostics
```

Do not duplicate the language profile across individual targets.

Foreign/unsafe targets may link a separate narrowly defined profile containing
only the exceptions required by that boundary.

### 3.6 Separate GCC and Clang graphs

Do not point clang-tidy at GCC-produced BMIs.

Maintain:

```text
build/gcc-dev/
build/gcc-release/
build/gcc-asan-ubsan/
build/gcc-tsan/
build/clang-lint/
```

The GCC graph includes every named module and all reflection code.

The Clang lint graph excludes every translation unit or module unit whose
parse graph contains unsupported reflection syntax. A named module with
reflection partitions is GCC-only as a unit, so the lint graph covers the
Clang-parseable remainder: `foreign/`/`unsafe/` translation units that import
no module.

The two graphs represent the same reflection-free source semantics; they are
not allowed to use conditional source code to create two different programs.

### 3.7 Dependency acquisition

Build dependencies must be immutable and reproducible.

Prefer, in order:

1. project-owned source,
2. a module-native dependency pinned to an immutable revision,
3. a pinned dependency adapted behind `foreign/`,
4. rewriting/replacing a dependency that would otherwise force exceptions,
   shared ownership, macro APIs, or header-centric design into dialect code.

If CMake acquires a dependency, pin it to an immutable revision. Never track a
moving branch.

External dependencies are consumed as targets. Do not propagate their
preprocessor configuration or header model into dialect modules.

### 3.8 No redundant build mechanisms

Do not add:

- custom compile scripts that duplicate CMake target compilation,
- manually generated compile databases,
- hand-managed BMI caches,
- a second package/build graph,
- per-developer build instructions that bypass presets.

If CMake/Ninja cannot express a required build operation cleanly, first
determine whether the operation belongs in `foreign/`, generated tooling, or a
small purpose-built repository tool. Do not grow a parallel build system.

---

## 4. Clang-tidy is a secondary frontend

GCC is authoritative for compilation.

clang-tidy is a separate Clang frontend. It may analyze only the module graph
that Clang can parse. Until Clang supports the reflection syntax used by this
repository, GCC-only reflection translation units are excluded from clang-tidy.

This does **not** weaken the language rules. GCC-only files obey this document
and are checked by compiler diagnostics plus review.

Maintain the dedicated `clang-lint` CMake/Ninja graph for translation units
whose entire parse/import graph is Clang-readable. A named module containing
reflection partitions is excluded from that graph as a unit until the pinned
Clang frontend supports the reflection syntax used by the project.
Build Clang-compatible BMIs there; never attempt to feed GCC BMIs to
clang-tidy.

Pin the clang-tidy version. Query-based custom checks are experimental; tool
upgrades require deliberate review of `.clang-tidy`, `-list-checks`, and
`--dump-config`.

Run query-based rules with:

```text
clang-tidy --experimental-custom-checks ...
```

A clang-tidy warning is a build failure.

`NOLINT`, `NOLINTNEXTLINE`, `NOLINTBEGIN`, and `NOLINTEND` are forbidden in
dialect code.

---

## 5. Source zones

The repository has two semantic zones.

### 5.1 `src/`, `tests/`, and `examples/`: dialect code

All rules in this document apply.

No preprocessor.
No headers.
No unsafe primitives.
No lint suppression.

Reflective code is ordinary dialect code and lives beside its callers. The
old `meta/` quarantine is retired because partitions make the whole module
GCC-only as a unit, so reflection syntax no longer needs its own directory to
keep the lint graph parseable. Code that clang-tidy cannot see may **not**
use the preprocessor, headers, exceptions, RTTI, raw allocation, inheritance,
shared ownership, raw threads, or other forbidden mechanisms merely because
only GCC checks it.

### 5.2 `foreign/` and `unsafe/`: quarantine

Only unavoidable machine/ABI adaptation lives here:

- OS and libc headers,
- vendor C APIs,
- compiler intrinsics,
- syscalls,
- atomics/locks used to implement higher-level primitives,
- pointer arithmetic required by an ABI or memory primitive,
- `reinterpret_cast` required by the boundary.

These directories may use headers and preprocessing when the external interface
requires them.

They must export a safe module partition or narrow ABI upward. No dialect
module unit may include a foreign header or depend on preprocessor state
transitively.

Do not move ordinary application code into `unsafe/` to escape a rule.

---

## 6. Modules and preprocessing

### Module structure

One importable named module per component. Its internals are partitions:

```cpp
// core.cc — one concern per partition file
export module project:core;

import std;
import :schema;
```

```cpp
// project.cc — the primary interface, the only export surface
export module project;

export import :core;
export import :schema;
```

- The primary interface does nothing but `export import` its partitions.
- Outsiders import the component module. They physically cannot import a
  partition, so the module name is the entire public surface — visibility is
  enforced by the language, not by convention.
- No dotted module names anywhere. Hierarchy is directories, not name
  prefixes; the partition name is the file stem (underscore-disambiguated on
  collision).
- One partition per concern, in one file (roughly 50-300 lines). Do not split
  a partition into interface and implementation units; split only on
  demonstrated rebuild pain.

### Project code

Dependencies between components are module imports:

```cpp
import std;
import project;
```

The single allowed project source extension is:

```text
.cc
```

Interface-ness is declared by the build (`FILE_SET CXX_MODULES`), never by
spelling; the extension is inert. Retired spellings, not to be reintroduced:

```text
.cpp
.cppm
```

Forbidden header-like extensions include:

```text
.h
.hh
.hpp
.hxx
.inc
.inl
.ipp
.tpp
```

Header units are forbidden:

```cpp
import <vector>;      // forbidden
import "thing.hpp";   // forbidden
```

Textual inclusion is forbidden in dialect code.

### Preprocessor

Every preprocessing directive is forbidden in dialect code (`src/`, `tests/`,
and `examples/`), including:

```text
#include
#define
#undef
#if
#ifdef
#ifndef
#elif
#else
#endif
#pragma
#error
#warning
#line
#embed
```

Command-line/toolchain macros may exist because the implementation uses them;
project source must not branch on them.

Platform selection and feature selection belong in the build graph.

---

## 7. Structural typing only

The project's semantic programming model is structural.

C++ itself retains nominal identity for user-defined types. Project APIs must
not use that identity as a substitute for an actual semantic requirement.

### Required

Express capabilities as minimal concepts:

```cpp
template<class T>
concept Readable =
    requires(T& value, Key key) {
        value.read(key);
    };
```

Conformance is implicit. A type satisfies a concept because its structure and
operations satisfy the requirement.

Prefer several small concepts to a giant pseudo-interface.

### Forbidden

Do not create nominal distinctions solely to make the compiler reject values:

```cpp
struct UserIdTag;
using UserId = Strong<UserIdTag, std::uint64_t>;  // forbidden philosophy
```

Do not use:

- marker base classes,
- marker structs,
- empty tag types,
- phantom tags whose only purpose is nominal identity,
- registration traits whose only job is “T implements X,”
- inheritance as interface declaration,
- CRTP as interface declaration,
- exact-type allowlists as semantic dispatch.

Do not write semantic dispatch like:

```cpp
if constexpr (
    std::same_as<T, Foo> ||
    std::same_as<T, Bar>
) {
    ...
}
```

Write the structural concept that explains why `Foo` and `Bar` are accepted.

`std::same_as` / exact type identity is permitted only when exact
representation is itself the requirement, for example ABI/meta implementation
checks or expression-result requirements inside a structural concept.

Physical representation and semantic compatibility are separate concepts.
Two physical `u64` values may be compatible or incompatible according to
compile-time context/laws without being wrapped in nominal IDs.

---

## 8. Data modeling

### Product types

Plain data is a public aggregate:

```cpp
struct User {
    std::uint64_t id;
    std::string name;
};
```

Do not create private fields plus trivial getters/setters.

Use designated initialization where practical.

### Sum types

Closed alternatives use:

```cpp
using Command = std::variant<Insert, Delete, Commit>;
```

Never use:

- inheritance hierarchies for closed alternatives,
- C unions,
- manual discriminator + payload structs,
- boolean matrices encoding states.

### Optionality

Use `std::optional<T>` only for genuine absence.

Do not encode absence as:

- zero,
- `-1`,
- empty string,
- null raw pointer,
- magic enum case unless that case is semantically real.

### Finite vocabularies

Use `enum class`.

Do not duplicate enum names into manual string tables if reflection can derive
the mapping.

---

## 9. `struct` and `class`

`struct` means transparent product data.

`class` is reserved for a real invariant, resource, or capability whose
constructor/destructor/access control makes invalid operations impossible or
owns a resource.

Good examples:

- file/socket handles,
- transaction capabilities,
- arena owners,
- scheduler handles,
- database handles.

Bad examples:

- “service” objects containing mutable business state,
- manager/factory hierarchies,
- Java-style DTOs,
- getter/setter shells,
- abstract interfaces.

Inheritance is forbidden even for classes.

---

## 10. No runtime OO or RTTI

Forbidden everywhere outside an unavoidable foreign ABI adapter:

```text
virtual
override
dynamic_cast
typeid
inheritance
CRTP
abstract base classes
visitor class hierarchies
```

Use:

- concepts/templates for open compile-time polymorphism,
- `std::variant` for closed runtime alternatives,
- reflection for structural metadata,
- one explicitly approved type-erasure boundary only when runtime openness is
  a real requirement.

Do not build plugin-like openness accidentally.

---

## 11. Exceptions do not exist

Production compilation disables exceptions.

Forbidden:

```text
throw
try
catch
exception_ptr
current_exception
rethrow_exception
```

No dependency that requires exceptions may leak an exception-based interface
into dialect code. Adapt it at a foreign boundary or replace/rewrite it.

### Error algebra

Use:

```text
std::expected<T, E>    recoverable synchronous failure
std::optional<T>       genuine absence
std::variant<...>      alternatives
sender error channel   asynchronous failure
sender stopped channel cancellation
C++26 contract         impossible programmer state
termination            unrecoverable process failure
```

There is exactly one mechanism per failure class: recoverable failure uses
`std::expected`; impossible programmer state uses a C++26 contract (`pre`,
`post`, `contract_assert`); compile-time law uses `static_assert`;
unrecoverable process failure terminates.

Never use error-code + output-parameter APIs in dialect code.

Do not use exceptions as hidden control flow inside implementations.

---

## 12. Ownership and borrowing

### Defaults

Prefer values.

Use `std::unique_ptr<T>` only when dynamic stable ownership is actually
required.

Use an arena when many objects share one obvious lifetime or stable storage.

### Forbidden

Dialect code may not contain raw pointer declarations or raw pointer return
types.

No:

```text
T*
void*
new
delete
malloc
calloc
realloc
free
shared_ptr
weak_ptr
enable_shared_from_this
```

No raw owning pointer exists.

### Borrows

Use exactly:

```text
T& / T const&        required single-object borrow
std::optional<T&>    optional single-object borrow
std::span<T>         contiguous sequence borrow
std::string_view     text borrow
```

The pinned toolchain implements C++26 `std::optional<T&>`. The
`std::optional<std::reference_wrapper<T>>` spelling is forbidden.

Do not invent alternate project-specific view wrappers without a demonstrated
semantic need not covered by these primitives.

### Lifetime rules

- Synchronous function parameters may borrow.
- Owning structs do not contain borrows unless the type is explicitly an
  ephemeral view product (`FooView` naming).
- Values crossing an async/sender boundary must be owned.
- A sender returned from a function must not capture locals or parameters by
  reference.
- Returned borrows must never derive from locals or by-value parameters.
- Messages between concurrent components must be ownership-closed: recursively
  free of references, `std::optional<T&>`, `std::span`, `std::string_view`, and
  `std::function_ref`.

---

## 13. Shared ownership is forbidden

`std::shared_ptr`, `std::weak_ptr`, `std::enable_shared_from_this`, and
`std::make_shared` are forbidden in dialect code.

If ownership is ambiguous, redesign the ownership graph.

Typical choices:

- value ownership,
- one unique owner,
- arena ownership,
- serialized actor/capability ownership,
- IDs/handles into an owning store.

Reference counting must not become the default answer to an unclear lifetime.

---

## 14. Mutation

Mutation is permitted when it is local and unobservable during construction or
an algorithm.

This is conceptually pure:

```cpp
auto build_index(Data const& data) -> Index {
    Index result;
    for (auto const& item : data) {
        result.insert(item);
    }
    return result;
}
```

Shared observable mutation is an effect and must be isolated.

Prefer:

```text
input value -> local mutation -> output value
```

over long-lived mutable object identity.

Non-const globals are forbidden.
Function-local statics are forbidden.
`thread_local` state is forbidden.

Pass dependencies, state, clocks, randomness, and schedulers explicitly.

---

## 15. Concurrency and async

The only application-level async/concurrency algebra is C++26
`std::execution` sender/receiver.

Use sender combinators to describe work as values.

Typical vocabulary:

```text
just
then
let_value
let_error
let_stopped
when_all
starts_on
continues_on
```

Direct coroutine syntax is forbidden in dialect code:

```text
co_await
co_yield
co_return
```

Also forbidden:

```text
std::thread
std::jthread
std::async
std::future
std::promise
std::packaged_task
detached work
ad-hoc callback concurrency
```

### Toolchain status

Sender/receiver is **active**: the pinned libstdc++ does not yet ship
`std::execution`, so the algebra is provided by the reference implementation
(NVIDIA stdexec, pinned to an immutable SHA in the top-level CMakeLists.txt
per §3.7) and quarantined behind exactly one swap boundary, re-exported as
`namespace ex`. The vocabulary rules above bind for real now — they are not
aspirational.

Boundary law:

- There is exactly ONE swap boundary, spelled across exactly two plain
  quarantine translation units: `foreign/exec.backend.cc` (the combinator
  half — `namespace ex` plus the conformance chains) and
  `unsafe/net.backend.cc` (the I/O half — the kqueue io-context and its
  readiness senders). Those two TUs are the only ones that may include a
  stdexec header or spell the implementation namespaces (`stdexec::`,
  `exec::`); the eventual swap edits only them. Direct
  implementation-namespace use anywhere else — dialect code, the rest of
  `unsafe/`, the rest of `foreign/` — is forbidden, and no third stdexec TU
  may be added: new sender work extends one of the two halves.
- `ex::` re-exports only the probe-verified combinator subset (`just`,
  `then`, `upon_error`, `let_value`, `let_error`, `when_all`,
  `continues_on`, `starts_on`, `schedule`, `on`, `into_variant`,
  `stopped_as_optional`, `just_error`, `just_stopped`,
  `static_thread_pool`) plus OUR expected-erroring `wait`. Nothing outside
  that list escapes the boundary.
- `stdexec::sync_wait` is forbidden: under `-fno-exceptions` its typed-error
  channel is silently lossy (routed through a null `exception_ptr`, so every
  error masquerades as stopped). `ex::wait<E>` returns
  `optional<expected<values, E>>` — nullopt is stopped, unexpected preserves
  the typed error.
- Pinned GCC 16.1 quirk: including a stdexec header in **any** module unit
  ICEs (global-module-fragment CPO forward-declaration pattern; rationale
  pinned in `foreign/exec.backend.cc`). Sender composition therefore cannot
  cross the module boundary on this toolchain: the `starter:exec` partition
  exports concrete function surfaces over a narrow ABI, not senders.
  Re-verify on every toolchain bump.
- The `__cpp_lib_senders` tombstone in `tests/conformance.test.cc` now
  signals time-to-delete-the-vendor: when the macro appears, rewrite the
  boundary over `std::execution`, drop the FetchContent pin, and re-export
  the full vocabulary directly.

### Shared mutable state

Application code does not own mutexes or atomics.

Mutable concurrent state lives behind:

- an actor,
- a serialized executor/capability,
- a database/transaction primitive,
- another small infrastructure abstraction with a single ownership model.

`std::mutex`, `std::shared_mutex`, `std::recursive_mutex`,
`std::condition_variable`, `std::atomic`, and explicit memory orders are
restricted to `unsafe/` concurrency primitives.

No lock may survive an async suspension boundary.

### Time and deadlines

Waiting is spelled with absolute deadlines, never relative timeouts.

- Any API that waits, expires, or retries with backoff takes and stores a
  `std::chrono::steady_clock::time_point` — a deadline — not a duration.
- Only the `*_until` spelling of a waiting primitive is dialect; the
  `*_for` family is forbidden. A relative timeout drifts: a wait that is
  interrupted and re-armed with its original duration extends the real
  deadline by the time already waited, invisibly, until a signal or
  spurious wakeup lands in production. A deadline cannot drift — the
  remaining time is recomputed (`deadline - now()`) at the last moment
  before each syscall, and re-arming is idempotent.
- Durations exist only at the edge where a deadline is minted:
  configuration may say "5 seconds"; the code that reads it converts
  once, immediately, and only the deadline travels.
- Deadlines use `steady_clock`, never `system_clock`: wall clocks jump.

---

## 16. Functional core

Prefer functions whose visible semantics are:

```text
A -> B
A -> expected<B, E>
A -> sender<B, E, stopped>
```

Prefer composition over mutation-heavy orchestration.

Use ranges and standard algorithms where they make the transformation clearer
than manual index/control loops.

Recursion is allowed. It is not intrinsically a defect.

Avoid hidden I/O, clocks, randomness, global caches, and implicit schedulers in
otherwise pure functions.

---

## 17. Reflection is the structural source of truth

If information already exists in declarations, derive it.

Use C++26 reflection for:

- field/member enumeration,
- enum enumeration,
- serialization,
- schema derivation,
- formatting,
- structural hashing,
- codecs,
- RPC/command metadata,
- CLI metadata,
- exhaustive generated visitors,
- compiler-facing DSL elaboration.

Do not duplicate the same structure in:

- macros,
- registration tables,
- manual field lists,
- generated boilerplate,
- parallel “descriptor” classes.

### Annotations

Annotations are allowed only for semantic information that cannot be derived
from the declaration itself.

Do not turn annotations into an alternate object model.

### Elaboration

Use `consteval` for compile-time semantic analysis that must succeed before a
program exists.

Use `static_assert` for compile-time invariants.

Prefer:

```text
declaration
  -> reflection
  -> consteval analysis
  -> constraint/check
  -> specialized code
```

over runtime metadata graphs.

---

## 18. Concepts, templates, and metaprogramming

Concepts are the public language of generic requirements.

Use named concepts when a requirement is semantically reusable.
Use local `requires` expressions for one-off structural requirements.

Forbidden as public dispatch mechanisms:

```text
std::enable_if
enable_if_t
SFINAE overload tricks
void_t detection idioms
tag dispatch
marker traits
trait registration
CRTP
inheritance
```

Type traits remain allowed for representation facts where they are actually
the question, e.g. trivially-copyable/layout/compiler implementation checks.

Template specialization is not a substitute for a concept. Use specialization
only for representation-level implementation or unavoidable foreign/standard
customization points.

---

## 19. No exact-type semantic dispatch

The following pattern is forbidden for semantic behavior:

```cpp
if constexpr (std::same_as<T, Foo>) {
    ...
} else if constexpr (std::same_as<T, Bar>) {
    ...
}
```

Instead define the structural property that distinguishes the behavior or use a
closed `std::variant` when the alternatives are semantically closed.

Exact type tests are allowed only when type identity is itself the
representation-level fact under examination.

---

## 20. No manual dynamic type containers

Forbidden in dialect code:

```text
std::any
std::type_info
std::type_index
void*
manual type tags
runtime registration by type identity
```

Choose:

- a structural concept,
- a reflected structure,
- `std::variant`,
- a specifically approved type-erasure abstraction.

---

## 21. Callable polymorphism

Compile-time callable polymorphism uses templates/concepts.

For a non-owning runtime-erased callable boundary, prefer C++26
`std::function_ref`.

`std::function` is forbidden.

Do not use `std::bind`; use lambdas.

Do not store callbacks merely to simulate an object/interface graph. Prefer
sender composition or explicit values.

---

## 22. Casting

Forbidden in dialect code:

```text
C-style casts
reinterpret_cast
const_cast
dynamic_cast
static downcasts
```

`static_cast` is permitted only for explicit, checked language conversions
where there is no clearer construction/conversion API. It is not a license for
nominal downcasting.

Prefer constructors, conversion functions, `std::bit_cast`, safe numeric
conversion helpers, or structural APIs as appropriate.

Pointer/integer conversion belongs only in `unsafe/foreign`.

---

## 23. Numeric conversions

Implicit narrowing is forbidden.

Signed/unsigned conversion must be intentional.

Do not silence conversion warnings with an unexplained cast. Use an operation
that states the range/checking policy.

Money, counts, offsets, lengths, timestamps, and physical quantities keep their
physical representation explicit. Do not manufacture nominal wrappers solely
to distinguish otherwise equal representations.

When semantic compatibility depends on context, encode that context in
compile-time laws/concepts rather than tag types.

---

## 24. C remnants

Forbidden in dialect code:

```text
C arrays
varargs
C unions
setjmp/longjmp
printf-family formatting
malloc-family allocation
qsort/bsearch-style erased callbacks
pointer + length APIs
null-terminated borrowed APIs
```

Use:

```text
std::array
std::span
std::variant
templates/concepts
std::format / std::print
RAII containers
ranges/algorithms
std::string_view
```

---

## 25. I/O and formatting

Use `std::print` for ordinary formatted output.

Use `std::format` for formatted strings.

Do not introduce iostream-based formatting (`std::cout`, `std::cerr`,
`std::stringstream`, stream insertion as the generic formatting protocol) in
dialect code.

Do not introduce printf-family formatting.

Binary I/O should use typed byte/span APIs and explicit codecs.

---

## 26. API shape

Public APIs should be explicit and narrow.

Default arguments are forbidden.

When optional configuration becomes nontrivial, pass an explicit aggregate
options value:

```cpp
struct OpenOptions {
    bool read_only;
    std::size_t cache_bytes;
};

auto open(Path path, OpenOptions options) -> std::expected<Db, OpenError>;
```

Avoid long positional parameter lists.

Return values instead of output parameters.

Use structured return values instead of tuples whose positions have domain
meaning when named fields materially improve the API.

### Return values are contracts

Every non-void function in dialect code is declared `[[nodiscard]]`.

If a result is safely ignorable, the function should return `void`: the
mutation is the result. The rare advisory return stays `[[nodiscard]]`;
callers acknowledge it explicitly.

Exceptions, set by the language's own conventions: operators whose result
is the object itself (assignment, compound assignment,
increment/decrement) and constructors. Immediately-invoked or
immediately-consumed lambdas need no attribute.

Discarding has exactly two spellings, one per concept:

```cpp
std::ignore = advisory_value();   // discard a value, deliberately
auto _ = scoped_capability();     // hold an unnamed RAII object to scope end
```

`(void)expr` is retired. The two spellings are not interchangeable:
`std::ignore =` destroys immediately; `auto _ =` lives to scope end.

An `expected` is never discarded by either spelling. Check it; then the
extracted value may be discarded if advisory. Ignoring an `expected`
ignores its error — a swallowed failure, not a style choice.

Enforcement is the compiler rung: `[[nodiscard]]` + `-Werror` fails the
build in every preset. The lint graph supplements with
`bugprone-unused-return-value` over `expected`/`optional` return types.

### Errors are decision surfaces

An error type exists so the caller can branch. Design it around the
caller's actual decision — and for a long-running program the universal
decision is retryability. Every exported error type answers it:

```cpp
[[nodiscard]] auto is_transient() const -> bool;
```

`is_transient()` answers exactly one question: *could the same operation,
retried later with nothing changed by the caller, plausibly succeed?*
Contention, peer disconnects, and resource pressure are transient;
protocol violations, structural misuse, corruption, and configuration
errors are permanent. When unsure, classify permanent: a caller that
gives up too early produces a bug report; a caller that retries a
permanent failure forever produces an outage.

Classification is exhaustive and compile-enforced where the
representation allows: a `switch` over the error enum with no `default`
(under `-Werror`, `-Wswitch`) means a new variant fails the build until
someone classifies it. For errno-carrying errors, unknown values are
permanent. The policy — which kind sits on which side, and why — is
pinned by a classification test and documented with the error type's
reference documentation, not narrated in the implementation.

Messages are rendering, never input: deciding behavior by matching an
error's text is forbidden. If callers need to distinguish two failures,
they are two kinds (or one kind with a typed payload), not two strings.

### Failure is transparent

A function that fails returns only the failure. On the `unexpected` path
(or a sender's `set_error` completion) every caller-visible output —
out-buffers, referenced targets, partially built state — is exactly as
the caller left it, unless the surface's documentation says otherwise in
so many words. `write_response` is canonical: the full size is computed
first, so a short buffer fails before a single byte lands.

The mirror rule for narrow-ABI error out-parameters (`err_stage`,
`err_code`): written on the failure path only; untouched on success.

### The code is the document

Prose is a last resort. The representable is represented:

- a capability or requirement -> a concept,
- a precondition, postcondition, or invariant -> `pre` / `post` /
  `contract_assert`,
- a compile-time law -> `static_assert` (with its message as the
  explanation),
- a valid-values story -> a type: an enum, a checked constructor, a
  distinct unit type,
- behavior -> a test that pins it,
- a name that would need explaining -> a better name.

A documentation block states only what is genuinely unrepresentable in
code: units and coordinate conventions, error meanings and the caller
decisions they drive, lifetime rules the type system cannot see, ABI and
protocol facts, the "when not to use this." If a concept, contract, type,
or test can carry the fact, prose may not.

Exactly three comment forms exist:

1. **The documentation block**, immediately above a declaration:

   ```cpp
   /**
    * Sum numbers in a vector.
    *
    * This sum is the arithmetic sum, not some other kind of sum that
    * only mathematicians have heard of.
    *
    * @param values Container whose values are summed.
    * @return sum of `values`, or 0.0 if `values` is empty.
    */
   ```

   Thin by law: say what the signature cannot, then stop. A block (or a
   `@param` line) that restates the signature is a violation. Exported
   declarations carry a block when they hold an unrepresentable fact;
   internal declarations almost never do. Multi-paragraph treatises
   belong in `docs/` — the block summarizes and points.

2. **`/* PIN(name): one line */`** at a pinned-workaround site — the
   license for code that is deliberately wrong by dialect law because
   the pinned toolchain requires it. Every PIN is backed by a PINS.md
   registry entry (symptom, affected sites, tombstone condition,
   upstream link); the essay lives there, once. The PIN's job is to stop
   the fix-the-violation reflex at the exact line — human or model.

3. **`/* SAFETY: one line */`** in `unsafe/` and `foreign/` only — the
   justification at each site whose soundness the type system cannot
   see (aliasing, lifetime across an ABI, thread-crossing).

`//` does not exist in this codebase. No file-header essays, no section
banners, no in-function narration, no `} // namespace` closers, no TODO
markers. Rationale lives in PINS.md, `docs/`, and commit messages;
contracts live at the interface in documentation blocks; everything else
lives in the code itself.

Why: prose in source is paid by every reader on every read — human or
model — whether it is needed or not, and it inflates the context required
to reason about the code; prose in docs is paid on demand. And prose
asserts nothing: comments rot silently, while names, concepts, contracts,
and tests break loudly.

Enforcement is review (§34): a comment that is not one of the three forms
is rejected on sight — that is what the fixed spellings are for. There is
no machine checker, by the same decision that bans repository grep
checks.

---

## 27. Boolean parameters and state flags

Avoid boolean parameters when they select qualitatively different behavior.

Prefer an enum/variant/options aggregate that makes the state explicit.

Do not represent state machines as many independent booleans.

Use a closed algebraic representation.

---

## 28. Resource classes

A resource `class` should normally be:

- non-copyable when ownership is unique,
- movable when movement is valid,
- destructible without failure,
- small,
- explicit about ownership,
- free of inheritance.

Destructors must not fail.

Move operations for resource holders should be `noexcept` when semantically
possible.

Do not expose raw native handles except inside `unsafe/foreign` adaptation.
When unavoidable, make the escape explicit and narrow.

---

## 29. Allocation

Application code never uses `new` or `delete`.

Prefer, in order:

1. values / automatic storage,
2. fixed-size containers (`std::array`, fixed-capacity structures),
3. `std::inplace_vector<T, N>` where a bounded capacity is semantically real,
4. ordinary standard containers when dynamic capacity is semantically needed,
5. arenas for bulk/stable ownership,
6. `std::indirect<T>` for value-semantic heap indirection without pointer
   identity,
7. `std::unique_ptr` for truly independent dynamic lifetime.

The pinned toolchain implements both `std::inplace_vector` and
`std::indirect`; they narrow the legitimate uses of `std::unique_ptr`.

Zero-allocation is a property to design and measure in hot paths, not a reason
to replace safe containers with pointer arithmetic.

No hidden shared reference counting.

### Steady state

A long-running serving loop has two phases, and allocation belongs to the
first one.

- **Setup** (startup, configuration, schema load): standard containers,
  freely. This phase may fail; failing here is cheap and honest.
- **Steady state** (the request loop): the discipline is reuse and
  bounds, not abstinence. Buffers are acquired once and reused — clearing
  is not freeing. Per-request dynamic needs come from per-worker
  arena-style storage reset between requests. Ad-hoc churn against the
  global allocator inside the hot loop is a design smell that needs a
  stated reason.

**Unbounded growth in the serving path is a bug.** Under memory
overcommit, graceful allocation-failure handling is fiction — the failure
arrives as a kill signal, not as anything the program can observe (§11:
allocation failure under `-fno-exceptions` is termination). What actually
protects a daemon is a flat footprint: a process whose steady-state
memory is a constant fixed at startup cannot leak, cannot fragment its
way upward, and cannot bloat into the OOM killer's threshold.

Every wire-facing quantity has a named maximum (`max_request_bytes`,
`max_header_count`) and buffers are sized from those constants. The bound
is a security property — an unbounded untrusted input is a
denial-of-service vector — independent of allocation policy. Where a
compile-time bound exists, format into the caller's buffer
(`std::format_to_n`, or size-then-write as `write_response` does);
returning an owning string from a bounded hot path is churn.

Zero allocation at steady state is not a law; bounded allocation is.
Measure with allocation counts (§30) when a hot path warrants it. An
arena primitive is added when the first real consumer appears, not
before.

---

## 30. Performance discipline

Abstractions are expected to optimize away when that is part of their design.

For critical paths:

- benchmark,
- inspect optimized assembly/IR when relevant,
- test allocation counts,
- test cache/layout assumptions,
- use sanitizers in non-production configurations.

Do not replace a clear value/structural abstraction with manual low-level code
without evidence.

If low-level code is required, isolate it behind a safe module boundary.

---

## 31. Contracts and assertions

Use the type system when a property can be encoded cheaply.

There is exactly one mechanism per failure class:

- recoverable input/domain failure -> `std::expected`,
- impossible programmer state -> a C++26 contract (`pre`, `post`,
  `contract_assert`),
- compile-time law -> `static_assert`,
- unrecoverable process failure -> termination.

No other assertion or invariant primitive exists.

Do not turn recoverable errors into assertions.
Do not turn programmer bugs into ordinary error plumbing merely to avoid a
clear invariant.

---

## 32. Testing

Tests obey the dialect. Do not introduce a macro-heavy test framework into
`tests/`.

Prefer a module-native minimal test harness using ordinary functions,
`std::expected`/`contract_assert`, property tests, and data-driven tests.

Required CI modes should include:

- optimized production build,
- warnings-as-errors build,
- GCC static analysis where useful,
- clang-tidy over the Clang-readable graph,
- AddressSanitizer + UndefinedBehaviorSanitizer build,
- separate ThreadSanitizer build,
- fuzz/property testing for parsers, codecs, storage boundaries, and protocol
  surfaces where appropriate.

Do not combine ASan and TSan into one configuration.

---

## 33. Suppressions and escape hatches

Do not suppress a diagnostic in place.

Forbidden in dialect code:

```text
NOLINT
NOLINTNEXTLINE
NOLINTBEGIN
NOLINTEND
diagnostic pragmas
warning-disable pragmas
```

If a rule genuinely cannot apply, one of these must be true:

1. the design should change,
2. the code belongs in `unsafe/`,
3. the code belongs in `foreign/`,
4. the rule itself is wrong and should be changed centrally.

Local exceptions create a second language and are not permitted.

Enforcement is clang-tidy itself plus review; suppression markers are a review
convention, not the subject of any repository checker.

---

## 34. Enforcement

The rules in this document are enforced by a ladder, strongest layer first:

1. **Compiler flags** make violations unrepresentable: `-fno-exceptions` and
   `-fno-rtti` remove exceptions and RTTI from the language itself.
2. **The build graph** rejects what it can: module lists are explicit, header
   units fail dependency scanning, and wrong toolchains fail configure.
3. **clang-tidy AST checks** enforce the semantic bans over the Clang-readable
   graph.
4. Everything visible only in raw source text — preprocessor directives, lint
   suppressions, comments, header-like file extensions — is a **review
   convention**, deliberately **not** machine-enforced. There is no repository
   grep/regex checker, by decision.

Accepted gap: a named module with reflection partitions is GCC-only as a unit
and escapes the AST checks until the pinned Clang parses the project's
reflection syntax; the lint graph covers the Clang-parseable remainder —
`foreign/`, `unsafe/`, and any non-importing translation units. There,
enforcement is compiler flags plus review.

The ladder has a direction. Every rule aspires upward: a review
convention is a rule still waiting for its mechanism, tolerated only
while no concept, contract, type, name, build rule, or check can carry
it. When one can, it must — and the prose version is deleted the same
day. Evaluate every proposed practice by the highest rung it can ever
occupy; a practice that can only ever be a convention needs a reason to
exist at all. This is also the adoption filter for outside wisdom: take
the rule only if this toolchain can hold it better than its author
could.

---

## 35. Dependency policy

Prefer dependencies that:

- expose a module-native interface, or can be wrapped cleanly,
- do not require exceptions in the dialect-facing API,
- do not require shared ownership in the dialect-facing API,
- do not impose macro configuration on consumers,
- do not leak raw pointer ownership.

Legacy/header/C dependencies may be quarantined in `foreign/`.

Do not weaken the dialect globally to make a dependency convenient.
Adapt or replace the dependency.

---

## 36. Generated code

Generated code is allowed only when the language/build cannot reasonably derive
the information with C++26 reflection/`consteval`.

Do not generate code for convenience when reflection can directly derive the
same structure.

Generated code lives under `generated/`, is never hand-edited, and must expose a
normal module boundary.

Generated code does not define the style of hand-written dialect code.

---

## 37. Review checklist

Before accepting code, ask in order:

1. Can this be a value instead of an identity-bearing object?
2. Can this be a `struct` instead of a `class`?
3. Can this requirement be structural instead of nominal?
4. Can a concept express the capability?
5. Can reflection derive this metadata?
6. Can `consteval` reject this earlier?
7. Can `variant` encode this state space?
8. Can `expected` encode this failure?
9. Can a sender encode this effect/concurrency?
10. Can ownership be unique or arena-based instead of shared?
11. Can a reference/span/view replace a pointer?
12. Can local mutation replace shared mutation?
13. Is this low-level operation actually confined to `unsafe/foreign`?
14. Does this introduce a second mechanism for something already solved?
15. Is the new abstraction visible in optimized code when it should disappear?
16. Is every representable fact represented — concept, contract,
    static_assert, type, test, name — and every remaining comment one of
    the three forms, carrying only what code cannot (§26)?
17. Is every wait an absolute deadline — `steady_clock::time_point`,
    `*_until` (§15)?
18. Does every exported error type classify retryability
    (`is_transient()`), exhaustively (§26)?
19. Is the serving path's footprint bounded — buffers reused, wire
    quantities capped by named maxima (§29)?

If a proposed design uses an older mechanism while a blessed mechanism can
express the same thing, reject it.

---

## 38. Canonical style examples

### Structural generic code

```cpp
template<class T>
concept SizedRange =
    requires(T const& value) {
        value.size();
        value.begin();
        value.end();
    };

auto count(SizedRange auto const& value) -> std::size_t {
    return value.size();
}
```

No inheritance and no explicit registration.

### Fallible transformation

```cpp
auto process(std::string_view input)
    -> std::expected<Result, Error>
{
    return parse(input)
        .and_then(validate)
        .and_then(execute);
}
```

No exceptions.

### Closed state machine

```cpp
using State = std::variant<Idle, Running, Failed>;

struct StepVisitor {
    Event event;

    auto operator()(Idle value) -> State { /* ... */ }
    auto operator()(Running value) -> State { /* ... */ }
    auto operator()(Failed value) -> State { /* ... */ }
};

auto step(State state, Event event) -> State {
    return std::visit(StepVisitor{std::move(event)}, std::move(state));
}
```

No state-class hierarchy. The eliminator is a plain product type with one
`operator()` per alternative — not an `overload{...}` set built by variadic
inheritance, which this document forbids. Reflection may later derive
exhaustive visitors.

### Concurrent composition

```cpp
auto page(UserId id) -> sender auto {
    return std::execution::when_all(
        load_user(id),
        load_settings(id),
        load_notifications(id)
    )
    | std::execution::then(
        [](auto user, auto settings, auto notifications) {
            return Page{
                .user = std::move(user),
                .settings = std::move(settings),
                .notifications = std::move(notifications),
            };
        }
    );
}
```

No manually coordinated threads or shared mutable state.

### Reflection-derived structure

```cpp
template<class T>
auto encode(Writer& writer, T const& value) -> void {
    template for (
        constexpr auto member :
        std::meta::nonstatic_data_members_of(^^T)
    ) {
        encode(writer, value.[:member:]);
    }
}
```

Do not duplicate the field list elsewhere.

---

## 39. Final principle

This repository is not trying to preserve every C++ idiom.

It intentionally treats C++26 as a substrate for a smaller language:

> structural, value-oriented, reflection-driven, compile-time elaborated,
> exception-free, ownership-explicit, functionally composed, and
> structured-concurrent.

Freedom to use an older C++ mechanism is not a project goal.

If the modern static mechanism can express the design, the older mechanism is
illegal.
