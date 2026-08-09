// §34 compile-fail: a closed handle used against ANOTHER closed relation
// (TODO_CPP §34's "closed handle from wrong vocabulary"). `Ticket.priority`
// references the "Priority" vocabulary; binding a "Kind" handle at it in
// a match record must fail constant evaluation with the pinned diagnostic
// naming the handle, its vocabulary, the coordinate, and the coordinate's
// vocabulary.
import std;
import bumbledb.types;
import bumbledb.meta.relation;
import bumbledb.meta.closed;
import bumbledb.meta.schema;
import bumbledb.meta.query;

inline constexpr auto Kind =
    bdb::closed<"Kind", "Deterministic", "CustomOperator">();
inline constexpr auto Priority =
    bdb::closed<"Priority", "Low", "Normal", "Urgent">();

struct TicketRow {
    [[=bdb::fresh]]
    std::uint64_t id;

    bdb::ref_to<Priority.id> priority;
};

inline constexpr auto Ticket = bdb::relation<"Ticket", TicketRow>;

inline constexpr auto Tickets = bdb::schema<"Tickets">(
    Priority,
    Kind,
    Ticket,
    bdb::contained(
        bdb::on(Ticket.priority),
        bdb::on(Priority.id)
    )
);

// The wrong vocabulary: a Kind handle at the Priority-referencing field.
inline constexpr auto Broken =
    bdb::query(Tickets).rule([](auto r) consteval {
        auto vars = r.vars(Ticket);
        return r
            .match(Ticket,
                {
                    .id = vars.id,
                    .priority = Kind.Deterministic,
                })
            .find({
                .id = vars.id,
            });
    });
