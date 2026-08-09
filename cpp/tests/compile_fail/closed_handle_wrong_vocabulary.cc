import std;
import bumbledb;

inline constexpr auto Kind = bdb::closed<"Kind", "Deterministic", "CustomOperator">();
inline constexpr auto Priority = bdb::closed<"Priority", "Low", "Normal", "Urgent">();

struct TicketRow {
	[[= bdb::fresh]] std::uint64_t id;

	bdb::ref_to<Priority.id> priority;
};

inline constexpr auto Ticket = bdb::relation<"Ticket", TicketRow>;

inline constexpr auto Tickets =
    bdb::schema<"Tickets">(Priority, Kind, Ticket, bdb::contained(bdb::on(Ticket.priority), bdb::on(Priority.id)));

inline constexpr auto Broken = bdb::query(Tickets).rule([](auto r) consteval {
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
