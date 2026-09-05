/**
 * Maintenance operations as Effects with the chapter 35 admin certainty:
 * mutating operations return `AdminOutcome<Value>` in A with E = never
 * (completed / not-started / outcome-unknown around a small immutable
 * operation reference derived BEFORE dispatch); read-only verification has
 * typed E. These are language wrappers over each operation's existing
 * protocol report and identity (database/epoch/root/barrier/operation) —
 * no generic admin journal, manufactured maintenance receipts, or shared
 * durable state machine is added. Interruption stays in Cause; the retained
 * operation reference resolves it through `migrationStatus`/`inspect`.
 */
import type { AdminOperations } from "#machine.ts"
import { log } from "#production.ts"

export const checkpoint: AdminOperations["checkpoint"] = log.admin.checkpoint
export const pinRestorePoint: AdminOperations["pinRestorePoint"] = log.admin.pinRestorePoint
export const releaseRestorePoint: AdminOperations["releaseRestorePoint"] = log.admin.releaseRestorePoint
export const rotateReceiptEpoch: AdminOperations["rotateReceiptEpoch"] = log.admin.rotateReceiptEpoch
export const retireReceipts: AdminOperations["retireReceipts"] = log.admin.retireReceipts
export const collectGarbage: AdminOperations["collectGarbage"] = log.admin.collectGarbage
export const backup: AdminOperations["backup"] = log.admin.backup
export const verifyBackup: AdminOperations["verifyBackup"] = log.admin.verifyBackup
export const restore: AdminOperations["restore"] = log.admin.restore
export const erase: AdminOperations["erase"] = log.admin.erase

export type { AdminIdentityOptions, AdminOperations, BackupDestination, TenantOpenOptions } from "#machine.ts"
