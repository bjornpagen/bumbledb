/**
 * @bjornpagen/bumbledb-log — durable named application commands over the
 * bumbledb core: LocalHistory (one LMDB transaction), HostedHistory (one
 * S3 HEAD over immutable decisions), sealed commands with retained refs,
 * published snapshots satisfying the core QueryReader, one native-backed
 * TenantCache, and explicit maintenance/migration operations. Effect-only;
 * the one protocol implementation is crates/bumbledb-log behind the shared
 * native runtime. Core types (ChangeSet, QueryReader,
 * DbError, …) are the peer @bjornpagen/bumbledb's own exports, never
 * re-exported here.
 */

export type { AdminIdentityOptions, BackupDestination } from "#admin.ts"
export {
	backup,
	checkpoint,
	collectGarbage,
	erase,
	pinRestorePoint,
	releaseRestorePoint,
	restore,
	retireReceipts,
	rotateReceiptEpoch,
	verifyBackup
} from "#admin.ts"
export type { ProtocolCode } from "#codes.ts"
export { protocolErrorCodes } from "#codes.ts"
export { Command } from "#command.ts"
export type { LogError, ProtocolReason } from "#errors.ts"
export { ProtocolError } from "#errors.ts"
export { HostedHistory, LocalHistory } from "#history.ts"
export type {
	CommandId,
	CommandRef,
	DatabaseIdentity,
	DecisionStamp,
	Freshness,
	OperationRef,
	ReadConsistency,
	StateStamp
} from "#identity.ts"
export {
	CommandDigest,
	DatabaseId,
	DecisionDigest,
	IncarnationId,
	OperationId,
	PlanSetDigest,
	parseCommandRef,
	parseDatabaseIdentity,
	parseDecisionStamp,
	parseSchemaId,
	parseStateStamp,
	ReceiptEpoch,
	RequestId,
	RootId,
	renderCommandRef,
	renderDatabaseIdentity,
	renderDecisionStamp,
	renderStateStamp,
	sameCommandRef,
	sameIdentity
} from "#identity.ts"
export type {
	CreationOptions,
	HistoryBinding,
	HostedBinding,
	HostedCreateOptions,
	HostedCredentials,
	HostedOpenOptions,
	HostedOrigin,
	LocalBinding,
	LocalCreateOptions,
	ReadOptions,
	RuntimeExpectation,
	SubmitOptions
} from "#options.ts"
export type {
	AbortReport,
	AccessMode,
	ActivationRef,
	ActivationReport,
	AdminOutcome,
	BackupReport,
	BackupVerification,
	CacheInspection,
	CacheSlotReport,
	ChangeSummary,
	CheckpointReport,
	CommandResult,
	CommandScalar,
	ErasureReport,
	GcReport,
	GeneratedMigrations,
	HistoryInspection,
	InitializeValue,
	LocalMaterializationHealth,
	MigrateValue,
	MigrationRef,
	MigrationStatus,
	PublicationPhase,
	ReceiptPolicyReport,
	ReceiptRetirementReport,
	ReceiptRotationReport,
	ResidualCopy,
	ResolveOutcome,
	RestorePointReport,
	RestoreReport,
	RootReleaseReport,
	SourceAccessReport,
	SubmitOutcome,
	TerminalOutcome,
	TerminalReceipt
} from "#outcome.ts"
export type {
	Command as CommandValue,
	CommandInput,
	History,
	HistoryBorrow,
	Precondition,
	PublishedSnapshot
} from "#surface.ts"
export type { TenantCacheOptions } from "#tenants.ts"
export { TenantCache } from "#tenants.ts"
