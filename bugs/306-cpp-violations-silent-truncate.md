# C++ `Error::violations()` silently truncates a partial citation list
- id: 306
- severity: low
- confidence: likely
- area: correctness
- components: cpp/src/error.cc
- status: open (do not fix)

## Summary

`bdb::Error::violations()` asks the bridge for `violation_count()`, then copies index `0..count`. If `violation(k)` returns empty for any `k < count`, the loop **breaks** and returns the prefix with no error. A `CommitRejected` host that treats “non-empty list” as “here are all citations” can act on an incomplete set; a host that treats empty-vs-nonempty as a boolean can even miss that later citations exist.

## Evidence

```250:267:cpp/src/error.cc
	[[nodiscard]] auto violations() const -> std::vector<Violation> {
		auto rendered = std::vector<Violation>{};
		auto const count = handle_.violation_count();
		rendered.reserve(count);
		for (auto index = std::size_t{0}; index != count; ++index) {
			auto copy = handle_.violation(index);
			if (!copy.has_value()) {
				break;
			}
			rendered.push_back(Violation{ ... });
		}
		return rendered;
	}
```

The comment on the method says “the complete rendered violation set”. Breaking on a mid-list miss is not completeness. Dialect law (`cpp/AGENTS.md` §26) also forbids swallowing a failure on an `expected`/`optional` fetch used as a copy of a counted sequence.

This is not an FFI lifetime bug: it is the dialect wrapper’s error handling around a counted enumeration. A healthy bridge should never return empty for `k < count`; the C++ API still has no way to report that the copy was short.

## Why this is a bug

`CommitRejected` is defined as the complete sealed set (same invariant as finding 302). Truncation without a typed error makes “I got 1 violation” indistinguishable from “there was 1 violation”. Host repair/retry against a partial set is the same failure mode as the Rust applier skipping extra keys.

## How to trigger / repro sketch

1. Produce a `CommitRejected` with N>1 citations (two distinct key statements).
2. Arrange `violation_count() == N` but `violation(1)` empty (malformed bridge payload, index mixup, or a failed copy).
3. `err.violations().size()` is 1, `kind()` is still `CommitRejected`.

Against a correct bridge this stays latent; the API still cannot distinguish a short copy from a complete one.

## Related

- Rust `Violations::as_slice` (complete after `seal`)
- Finding 302 (incomplete set on the engine side)
- `cpp/foreign` violation accessors (bridge; not diagnosed here as UAF)
