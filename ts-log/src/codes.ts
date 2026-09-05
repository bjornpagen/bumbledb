/**
 * The log-specific error-code roster. This list must stay pinned to the
 * native `logErrorCodes()` export (one speller, the internal Rust machine);
 * an authored roster test compares them exactly. Core failures are never
 * respelled here: a core failure crossing the log surface remains the exact
 * core `DbError` with its own roster. These codes are only the protocol's
 * additions.
 */
export const protocolErrorCodes = [
	// Identity and authority refusals (chapter 20/30).
	"ForeignIdentity",
	"CommandIdentityConflict",
	"DatabaseDeleted",
	"DatabaseFrozen",
	"CommandEpochClosed",
	"ReceiptExpiredUnknown",
	"NotInitialized",
	"DatabaseMissing",
	"AuthorityExists",
	"CacheIdentityMismatch",
	// Read-consistency refusals (chapter 30 published reads).
	"WrongLineage",
	"NotYetAvailable",
	"WitnessUnavailable",
	"SnapshotExpired",
	// Maintenance and retention (chapters 21/22).
	"MaintenanceRequired",
	// The warm local materialization is behind the checkpoint base (older
	// than the retained tail): recovery hydration is required. Native owns
	// hydration — the reason's detail routes the caller to reopen the
	// history/tenant (which runs recovery), never to a JS repair path.
	"MaterializationStale",
	"RootCapacityExceeded",
	"SlotBorrowed",
	"Contention",
	"IncompleteRejectionEvidence",
	// Migration workflow (chapters 22/33).
	"MigrationRequired",
	"MigrationDrift",
	"MigrationIntentRequired",
	"MigrationUnsupported",
	"MigrationRepository",
	"DatabaseAhead",
	"MigrationOutputMismatch",
	"OperationConflict",
	// Host feasibility and transport (chapters 31/21).
	"InsufficientLocalDisk",
	"UnsupportedArtifact",
	"Corruption",
	"Backend",
	"Misuse"
] as const

export type ProtocolCode = (typeof protocolErrorCodes)[number]
