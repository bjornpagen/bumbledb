// :handle — closed-vocabulary member handles and the closed-reference
// value type (TODO_CPP §8): `Kind.DirectPass` is a handle_value, a
// closed-reference column stores a closed_ref, and the §34
// wrong-vocabulary wall lives on the converting constructor.
export module bumbledb:handle;

import std;
import :name;

export namespace bdb {

/// One member handle of a closed vocabulary (TODO_CPP §8): `Kind.DirectPass`
/// is a value of this type — the roster, the handle name, and the
/// declaration-order row id all ride the TYPE, so handle uses resolve and
/// roster-check during constant evaluation (queries resolve handles
/// HOST-side — lowering.md §7.8).
template<name_text Roster, name_text Handle, std::uint64_t Index>
struct handle_value {
    static constexpr name_text roster_name = Roster;
    static constexpr name_text handle_name = Handle;
    static constexpr std::uint64_t index = Index;

    [[nodiscard]] constexpr auto roster() const -> std::string_view {
        return roster_name.view();
    }
    [[nodiscard]] constexpr auto name() const -> std::string_view {
        return handle_name.view();
    }
};

} // namespace bdb

export namespace bdb::detail {

/// The §34 wrong-vocabulary wall's message (the closed-reference twin of
/// the cross-class walls): names the handle, its vocabulary, and the
/// reference's vocabulary.
consteval auto handle_crosses_vocabulary_message(name_text handle_roster,
    name_text handle, name_text reference_roster) -> std::string {
    return std::string{"bumbledb closed reference: handle \""}
        + std::string{handle.view()} + "\" belongs to closed relation \""
        + std::string{handle_roster.view()}
        + "\" but the reference's vocabulary is \""
        + std::string{reference_roster.view()}
        + "\" — a handle binds only its own closed relation";
}

} // namespace bdb::detail

export namespace bdb {

/// A closed-reference column's value type (TODO_CPP §8): `bdb::ref<Kind.id>`
/// dealiases to this. Physically the engine's u64 handle row id; the TYPE
/// carries the vocabulary, so a foreign handle cannot cross into the field
/// (the §34 wrong-vocabulary wall lives on the converting constructor).
/// The newtype label the wire carries stays LAW-COMPUTED (lowering.md §3);
/// this type never becomes a user-declared domain wrapper.
template<name_text Roster>
struct closed_ref {
    static constexpr name_text roster_name = Roster;

    std::uint64_t row{};

    closed_ref() = default;

    /// A handle of the same vocabulary IS the value (`.priority =
    /// Priority.Urgent`); a foreign handle is the pinned §34 diagnostic.
    template<name_text HandleRoster, name_text Handle, std::uint64_t Index>
    consteval closed_ref(handle_value<HandleRoster, Handle, Index>)
        : row{Index} {
        static_assert(HandleRoster == Roster,
            detail::handle_crosses_vocabulary_message(
                HandleRoster, Handle, Roster));
    }

    constexpr auto operator==(closed_ref const&) const -> bool = default;
};

template<class T>
inline constexpr bool is_closed_ref_v = false;

template<name_text Roster>
inline constexpr bool is_closed_ref_v<closed_ref<Roster>> = true;

} // namespace bdb
