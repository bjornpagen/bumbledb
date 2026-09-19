# Equal completed cofactors: a normalization control

The equal-cofactor shortcut is correct but does not repair the measured growth
or the canonical-reimport caps. Both modes remain available; `staged` stays the
default. All four shared suites and 32 native acceptance processes pass.
The [completed information-readout round](results/readout-final-audit.json)
remains frozen; none of its timings belong to this new source revision.

The preceding readout experiment exposed two separate problems: severe timed
general-fallback cost and node caps during untimed canonical reimport. Both
paths use the completed-root normalizer, but the cap backtraces identify only
the latter. The first control changes that normalizer while preserving the
decoder, canonical representation, algebra and source-law interpretation.

## The small algebraic change

After completing the low and high cofactors of a raw function, the normalizer
currently reconstructs the result as:

```text
(low & !selector) | (high & selector)
```

The raw input's cofactors can be distinct while their legal completions coincide.
The `equal` policy first checks their exact canonical raw IDs. When they agree,
it returns the shared completed cofactor. Otherwise it executes the existing
three-operation reconstruction. Both paths retain the same normalization memo
key, complement handling and immutable decoder.

This is the ordinary identity `ITE(selector, a, a) = a`, applied after legal
completion. It assumes neither probabilistic independence nor equality of
unrelated supported Events. The compared IDs belong to the same raw arena.
The [existing completion proofs](lean/Retraction.lean) retain the semantic
contract; this control does not add another public Event representation.

```text
EVENT_LAB_RETRACTION_NORMALIZE=staged|equal
```

`staged` remains the default. Both modes are compiled into the same executable.
Constructor, projection fallback and canonical reimport all use the selected
mode, so construction and query phases must remain separately visible.

## Checks and measurement contract

Each small retraction verifier now additionally clones its actual owner, flips
the normalization policy, clears the normalization memo, and reimports 160
inputs. It must recover exactly the already published root IDs. This runs for
all three test cutoffs under both decoders. The original owner is unchanged;
the check does not measure allocation savings in that prepopulated clone.

The shared suite retains the finite/symbolic information checks, arbitrary
support fallbacks, maps, exact counts/laws and publication checks. Both stores
and both policies must pass. Rust remains frozen throughout all compilation
and checking; native timings start only after those processes exit.

The [equal/slab](results/completion-equal-slab-check.json),
[staged/slab](results/completion-staged-slab-check.json),
[equal/enum](results/completion-equal-enum-check.json), and
[staged/enum](results/completion-staged-enum-check.json) suites pass. Each records
160 cross-policy canonical imports for six decoder/cutoff combinations, in
addition to the complete existing shared checks. These are correctness runs,
not performance measurements.

`completion_sweep.py` uses the same complete readout query and independent
oracle as the frozen round. `completion_comparison.py` makes normalization a
key dimension and compares only equal domain/mask/face queries. Dense, packed
and anchored controls use the staged label because they do not use this
normalizer. The older readout collector refuses experimental normalization
rows rather than silently combining the two modes.

Begin with the fibred bit-major high-X-bit query, then the four retained
canonical-reimport cap cases. Include the low-bit fast path, full support where
completion is identity, both physical orders, and the holed half-X/Y query.
Keep all incomplete processes and charge retained nodes/caches. If the shortcut
does not repair the growth, retain that negative result and compare a direct
ternary ITE kernel before claiming a need to change the resident representation.

A later fused projection/substitution kernel has the exact target recorded in
[the readout report](PREFIX-READOUTS.md#the-next-fallback-experiment-has-a-precise-algebraic-target).
That larger kernel is not implemented by this equal-cofactor control.

## Native result: retain this negative control

The [same-executable comparison](COMPLETION-MEASUREMENTS.md) retains 24 processes:
16 passed and eight capped, with no semantic assertion failures. The completed
processes supply 40 configurations and 28 matched comparisons. All twenty
normalization pairs have exactly equal input/final node counts, estimated bytes,
storage breakdowns and outer-memo bytes.

On the difficult width-six fibred high-X-bit query, staged takes 1660.009 ms and
equal takes 1742.380 ms in this one-sample screen. Both end with 1,363,847 nodes
and 162,956,364 estimated retained bytes. The screen does not establish a small
latency difference; it establishes that this shortcut removes none of that
retained growth. All four formerly capped configurations cap under both modes.
[Eight diagnostic backtraces](results/completion-cap-diagnostics.json) again
locate every cap in untimed canonical reimport, not the timed readout.

[Native acceptance](results/completion-acceptance.json) passes all 32 processes,
covering both decoders, both cutoffs, both policies, and materialized/mapped
products: 96 owned relation rows and 48 symbolic relation rows. No representation
selection follows. The next experiment is direct ternary ITE, with answer-root
reachability measured separately from arena retention, before a fused
projection/substitution kernel.
