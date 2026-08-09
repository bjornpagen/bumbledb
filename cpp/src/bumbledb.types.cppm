// bumbledb.types — the SDK's closed value vocabulary (TODO_CPP §7, §21).
//
// The Clang-visible, reflection-free half of the relation model: the row
// representation types the meta layer classifies, the checked value types
// the engine's Value roster demands, and the annotation object the
// reflector matches. This module is part of the lint graph; nothing here
// may use reflection syntax.
//
// Checked construction is dual-lane (TODO_CPP §21): a consteval `literal`
// factory for constants — an invalid constant is a compile error — and a
// constexpr `make` factory returning std::expected for runtime values, so
// the bridge can never present an unrepresentable value to the engine.
//
// Why the consteval lane's failure is not a contract_assert: the pinned
// Clang 22 lint frontend does not implement C++26 contracts, and this
// module is compiled by BOTH graphs as one source semantics (AGENTS.md
// §3.6). The invalid-literal path therefore uses the static failure
// spelling: the failure branch names a declared-but-undefined
// non-constexpr function, which no constant evaluation can satisfy, so the
// compile error reads as that function's name. Revisit when the pinned
// Clang gains P2900.
export module bumbledb.types;

import std;

namespace bdb::detail {

// Never defined; see the module comment. Reaching one of these in a
// consteval factory makes the invalid constant a compile error whose
// diagnostic carries the function name.
auto interval_literal_must_satisfy_lo_less_than_hi() -> void;
auto interval_literal_must_match_the_declared_width() -> void;
auto allen_mask_literal_must_fit_the_low_13_bits() -> void;

} // namespace bdb::detail

export namespace bdb {

/// Recoverable construction failure of a checked vocabulary value
/// (the runtime `make` lanes' error type).
enum class TypeError : std::uint8_t {
    /// interval: half-open [lo, hi) requires lo < hi.
    EmptyInterval,
    /// fixed-width interval: hi - lo must equal the declared width.
    IntervalWidth,
    /// allen_mask: bits above the low 13 are unrepresentable.
    AllenMaskOverflow,
};

/// The two interval element domains the engine's Value roster carries
/// (IntervalU64 / IntervalI64). Exact representation is the requirement.
template<class T>
concept IntervalElement =
    std::same_as<T, std::uint64_t> || std::same_as<T, std::int64_t>;

/// A checked half-open interval [lo, hi), strictly lo < hi — the C++ twin
/// of the engine's `Interval` (stored `Value` intervals are checked at
/// construction; TODO_CPP §21).
///
/// `Width` is the FIXED-WIDTH family label (the TS `interval(u64, 1n)`,
/// lowering.md §1.8): 0 is the general 16-byte interval; a nonzero width
/// makes `hi - lo == Width` part of the TYPE — a wrong-width value is
/// unconstructible host-side (the engine re-judges at commit either way),
/// and the width is a fingerprint input carried by the field's ValueType.
template<IntervalElement T, std::uint64_t Width = 0>
class interval {
    T lo_;
    T hi_;

    constexpr interval(T lo, T hi) : lo_{lo}, hi_{hi} {}

    static constexpr auto width_holds(T lo, T hi) -> bool {
        if constexpr (Width == 0) {
            return true;
        } else {
            return static_cast<std::uint64_t>(hi) - static_cast<std::uint64_t>(lo)
                == Width;
        }
    }

public:
    /// The declared width label (0 = the general interval).
    static constexpr std::uint64_t width = Width;

    /// The constant lane: an invalid literal is a compile error.
    static consteval auto literal(T lo, T hi) -> interval {
        if (!(lo < hi)) {
            detail::interval_literal_must_satisfy_lo_less_than_hi();
        }
        if (!width_holds(lo, hi)) {
            detail::interval_literal_must_match_the_declared_width();
        }
        return interval{lo, hi};
    }

    /// The runtime lane: an invalid pair is a typed recoverable error.
    static constexpr auto make(T lo, T hi)
        -> std::expected<interval, TypeError> {
        if (!(lo < hi)) {
            return std::unexpected{TypeError::EmptyInterval};
        }
        if (!width_holds(lo, hi)) {
            return std::unexpected{TypeError::IntervalWidth};
        }
        return interval{lo, hi};
    }

    [[nodiscard]] constexpr auto lo() const -> T { return lo_; }
    [[nodiscard]] constexpr auto hi() const -> T { return hi_; }

    // Member (not hidden-friend) comparison: the pinned GCC 16.1 ICEs
    // streaming a defaulted friend operator== across a module import.
    constexpr auto operator==(interval const&) const -> bool = default;
};

/// Fixed-width raw bytes: `bdb::bytes<N>` IS `std::array<std::byte, N>`
/// (one type, two spellings — TODO_CPP §7). The engine admits 1 ≤ N ≤ 64;
/// the relation reflector enforces that bound at classification.
template<std::size_t N>
using bytes = std::array<std::byte, N>;

/// The fresh-mark annotation's tag type. The relation reflector matches
/// annotations of this type; `fresh` below is the one annotation object.
struct FreshTag {};

/// The fresh mark: `[[=bdb::fresh]]` on a `std::uint64_t` row field marks
/// the engine-minted identity column (TODO_CPP §6; u64-only, enforced by
/// the reflector and re-judged by engine validation).
inline constexpr auto fresh = FreshTag{};

/// A set of Allen basic relations: a checked 13-bit mask, bit i = basic i
/// in the engine's palindromic bit order — before, meets, overlaps,
/// starts, during, finishes, equals, finished-by, contains, started-by,
/// overlapped-by, met-by, after (crates/bumbledb-theory/src/allen.rs; the
/// bit order is a specified representation). Query bind-time vocabulary
/// only — never a row field type (cpp/docs/lowering.md §1.6).
class allen_mask {
    std::uint16_t bits_;

    constexpr explicit allen_mask(std::uint16_t word) : bits_{word} {}

public:
    /// The all-13-bits word.
    static constexpr auto all_bits = std::uint16_t{(1U << 13U) - 1U};

    /// The constant lane: a bit above the low 13 is a compile error.
    static consteval auto literal(std::uint16_t word) -> allen_mask {
        if ((word & ~all_bits) != 0) {
            detail::allen_mask_literal_must_fit_the_low_13_bits();
        }
        return allen_mask{word};
    }

    /// The runtime lane: a bit above the low 13 is a typed error.
    static constexpr auto make(std::uint16_t word)
        -> std::expected<allen_mask, TypeError> {
        if ((word & ~all_bits) != 0) {
            return std::unexpected{TypeError::AllenMaskOverflow};
        }
        return allen_mask{word};
    }

    /// The raw 13-bit word (what crosses the ABI as `bdb_value.allen_mask`).
    [[nodiscard]] constexpr auto bits() const -> std::uint16_t {
        return bits_;
    }

    /// Mask union — the closed composition the named vocabulary is built
    /// from; preserves the low-13 invariant by construction.
    friend constexpr auto operator|(allen_mask left, allen_mask right)
        -> allen_mask {
        return allen_mask{
            static_cast<std::uint16_t>(left.bits_ | right.bits_)};
    }

    // Member (not hidden-friend) comparison: the pinned GCC 16.1 ICEs
    // streaming a defaulted friend operator== across a module import.
    constexpr auto operator==(allen_mask const&) const -> bool = default;
};

/// The engine's Allen vocabulary, under Allen's own names — the 13
/// singletons in bit order plus the engine's named compositions
/// (crates/bumbledb-theory/src/allen.rs).
namespace allen {

inline constexpr auto before = allen_mask::literal(1U << 0U);
inline constexpr auto meets = allen_mask::literal(1U << 1U);
inline constexpr auto overlaps = allen_mask::literal(1U << 2U);
inline constexpr auto starts = allen_mask::literal(1U << 3U);
inline constexpr auto during = allen_mask::literal(1U << 4U);
inline constexpr auto finishes = allen_mask::literal(1U << 5U);
inline constexpr auto equals = allen_mask::literal(1U << 6U);
inline constexpr auto finished_by = allen_mask::literal(1U << 7U);
inline constexpr auto contains = allen_mask::literal(1U << 8U);
inline constexpr auto started_by = allen_mask::literal(1U << 9U);
inline constexpr auto overlapped_by = allen_mask::literal(1U << 10U);
inline constexpr auto met_by = allen_mask::literal(1U << 11U);
inline constexpr auto after = allen_mask::literal(1U << 12U);

/// The point-sets share a point — the 9 middle bits (under half-open
/// intervals, meets shares no point).
inline constexpr auto intersects = overlaps | starts | during | finishes
    | equals | finished_by | contains | started_by | overlapped_by;

/// Point-set ⊇: equals ∪ contains ∪ started-by ∪ finished-by.
inline constexpr auto covers = equals | contains | started_by | finished_by;

/// Point-set ⊆ — covers' converse: equals ∪ during ∪ starts ∪ finishes.
inline constexpr auto covered_by = equals | during | starts | finishes;

/// The point-sets share no point: before ∪ meets ∪ met-by ∪ after — and
/// the pointwise key judgment's per-pair statement.
inline constexpr auto disjoint = before | meets | met_by | after;

/// All 13 basics — a value of the algebra; vacuous as a condition (the
/// query boundary rejects it, engine-side).
inline constexpr auto full = allen_mask::literal(allen_mask::all_bits);

/// No basic — likewise a value, vacuous as a condition.
inline constexpr auto empty = allen_mask::literal(0U);

} // namespace allen

} // namespace bdb
