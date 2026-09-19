# Coordinate permutations inside packed terminals

This experiment isolates an avoidable cost in the packed carrier. The original
permutation routine Shannon-decomposes every visited table and rebuilds its
characteristic function through canonical selection. That is necessary when a
map moves a table coordinate into the symbolic prefix. It is unnecessary when
the tail's entire coordinate set maps to itself.

Let the terminal's local axes be `T = [t0, ..., tk-1]`, where local table index
bit `i` denotes semantic coordinate `ti`. For a checked global bijection `p`,
the fast path exists precisely when every `p(ti)` lies in `T`. Then

```text
local_destination[i] = index of p(ti) in T
```

is a bijection of the local axes. Applying those axis swaps to the packed table
gives exactly the renamed cofactor. Global bijectivity also ensures no prefix
coordinate enters the tail under that condition. The prefix can still reorder;
it continues to use canonical selection. A map that crosses the cut retains the
general recursive algorithm. This is a checked property of the coordinate map,
not a guess based on input density or observed event values.

The word permutation uses the same broadword `Permutation::dense` kernel as
the dense carrier. Swaps wholly within a word use XOR/mask/shift operations;
cross-word swaps exchange selected bit fields or whole words. The active table
slice handles small partial terminals; unused storage stays zero. The result
returns through `leaf()` and its complement normalization and full-content
interning. Public permutation still recovers a true root, performs substitution,
and reseals against admissibility and the anchor.

Both paths are selected by an immutable flag read during arena construction:
`EVENT_LAB_PACKED_MAP=local` or `recursive`. There is one compiled binary and one
representation. Both pay literal construction and the normal public support
work; the local path additionally pays the invariance check and local-map setup.
The flag is experimental runner configuration, not a public Event API.

`map_sweep.py` verifies both modes, shuffles all packed algorithm/layout pairs
alongside the other carriers, and runs them serially. Each process executes the
same full relation query and checks all eighty results against matrices. The
comparison includes construction, fresh and warm arenas, outer memo on/off,
coordinate movement, residuals, closure, output counts, and retained memory.

This does not test a new packed representation, eliminate bad coordinate orders,
or establish that larger terminals are universally better. It tests whether a
specific apparent representation loss was caused by doing unnecessary work
inside a canonical coordinate transformation. The earlier recursive sources and
timings remain retained independently of the selectable control.
