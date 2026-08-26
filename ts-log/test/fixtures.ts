import {
	bool,
	capacity,
	closed,
	contained,
	interval,
	key,
	on,
	ref,
	relation,
	schema,
	span,
	str,
	u64,
	weigh,
	within
} from "@bjornpagen/bumbledb"

const Holder = relation("Holder", { id: u64.fresh, name: str })
const Booking = relation("Booking", { id: u64.fresh, holder: u64, slot: str, at: interval(u64) })
const Note = relation("Note", { id: u64.fresh, body: str })

const Ledger = schema("Ledger", { Holder, Booking, Note }, [
	key(Booking, ["slot"]),
	contained(on(Booking, "holder"), on(Holder, "id")),
	capacity(on(Holder, "id"), within(0n, 3n), on(Booking, "holder"))
])

const Pool = relation("Pool", { id: u64.fresh, supply: u64 })
const Device = relation("Device", { id: u64.fresh, pool: u64, watts: u64 })

const Grid = schema("Grid", { Pool, Device }, [
	capacity(on(Pool, "id"), weigh("watts"), within(0n, ref("supply")), on(Device, "pool"))
])

const Status = closed("Status", ["Open", "Frozen"])
const Kind = closed(
	"Kind",
	["DirectPass", "Failed"],
	{ mastered: bool, weight: u64, span: interval(u64) },
	{
		DirectPass: { mastered: true, weight: 2n, span: span(1n, 3n) },
		Failed: { mastered: false, weight: 5n, span: span(3n, 5n) }
	}
)

const Account = relation("Account", { id: u64.fresh, status: Status.id, kind: Kind.id })

const Vocab = schema("Vocab", { Status, Kind, Account }, [
	contained(on(Account, "status"), on(Status, "id")),
	contained(on(Account, "kind"), on(Kind, "id"))
])

export { Account, Booking, Device, Grid, Holder, Kind, Ledger, Note, Pool, Status, Vocab }
