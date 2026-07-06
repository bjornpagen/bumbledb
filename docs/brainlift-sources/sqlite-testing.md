# SQLite — How SQLite is tested (testing.html)

Source: https://www.sqlite.org/testing.html — fetched 2026-07-06

## The numbers (v3.42.0)
- Core: 155.8 KSLOC. Test code: 92,053 KSLOC — 590x test-to-source ratio.
- TH3 (proprietary): 100% branch and 100% MC/DC coverage of the core;
  ~2.4M test instances per run; ~248.5M in the pre-release soak.
- TCL suite: 51,445 cases. SQL Logic Test: 7.2M queries compared against
  PostgreSQL/MySQL/MSSQL/Oracle — a differential oracle across engines.
- dbsqlfuzz: mutates SQL AND the database file simultaneously; ~1B
  mutations/day; ended external fuzzer findings.
- 6,754 assert()s; OOM-injection loops; VFS I/O-error loops; crash tests
  that damage unsynced writes then check atomicity + integrity_check;
  ALWAYS()/NEVER() macros with three build modes that must agree.
- "Whenever a bug is reported... not considered fixed until new test
  cases that would exhibit the bug have been added."
- Static analysis verdict: "More bugs have been introduced into SQLite
  while trying to get it to compile without warnings than have been
  found by static analysis."

## Relevance to bumbledb
- SQL Logic Test is the ancestor of our verify oracle — differential
  testing against other engines as the exactness ground truth. Ours adds:
  the stamp (binary-fingerprint-keyed), refusal-to-time-unverified, and
  set-semantics normalization in the templates.
- The MC/DC-vs-fuzzing tension they document maps to our choice:
  representation-first (fewer branches to cover) + typed corruption
  errors + a small enough surface that the oracle covers query classes
  exhaustively per commit.
