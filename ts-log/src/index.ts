/**
 * @bjornpagen/bumbledb-log — durable named application commands over the
 * bumbledb core: LocalHistory (one LMDB transaction), HostedHistory (one
 * S3 HEAD over immutable decisions), sealed commands with retained refs,
 * published snapshots satisfying the core QueryReader, one native-backed
 * TenantCache, and explicit maintenance/migration operations. Effect-only;
 * the one protocol implementation is crates/bumbledb-log behind the shared
 * native runtime. Core types (ChangeSet, QueryReader, ExecutionPolicy,
 * DbError, …) are the peer @bjornpagen/bumbledb's own exports, never
 * re-exported here.
 */
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
export type { AdminIdentityOptions, BackupDestination } from "#admin.ts"
export type { ProtocolCode } from "#codes.ts"
export { protocolErrorCodes } from "#codes.ts"
export { Command } from "#command.ts"
export type { LogError, ProtocolReason } from "#errors.ts"
export { ProtocolError } from "#errors.ts"
export { HostedHistory, LocalHistory } from "#history.ts"
export type {
	CommandDigest,
	CommandId,
	CommandRef,
	DatabaseId,
	DatabaseIdentity,
	DecisionDigest,
	DecisionStamp,
	Freshness,
	IncarnationId,
	OperationId,
	OperationRef,
	PlanSetDigest,
	ReadConsistency,
	ReceiptEpoch,
	RequestId,
	RootId,
	StateStamp
} from "#identity.ts"
export {
	CommandDigest,
	DatabaseId,
	DecisionDigest,
	IncarnationId,
	OperationId,
	parseCommandRef,
	parseDatabaseIdentity,
	parseDecisionStamp,
	parseSchemaId,
	parseStateStamp,
	PlanSetDigest,
	ReceiptEpoch,
	renderCommandRef,
	renderDatabaseIdentity,
	renderDecisionStamp,
	renderStateStamp,
	RequestId,
	RootId,
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
	LocalOpenOptions,
	ReadOptions,
	RuntimeExpectation,
	SubmitOptions
} from "#options.ts"
export type {
	AccessMode,
	ActivationRef,
	ActivationReport,
	AbortReport,
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
export { TenantCache } from "#tenants.ts"
export type { TenantCacheOptions } from "#tenants.ts"
