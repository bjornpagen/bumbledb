import type * as Binding from "./binding.d.ts"
import { loadAddon } from "./load.ts"

/** The addon's exports, typed by the bridge's generated declarations. */
type Addon = typeof Binding

type External<T> = Binding.ExternalObject<T>
type RuntimeRef = External<Binding.RuntimeHandle>
type OperationRef = External<Binding.OperationHandle>
type DirectoryRef = External<Binding.DirectoryHandle>
type DbRef = External<Binding.DbHandle>
type SchemaRef = External<Binding.SchemaHandle>
type QueryRef = External<Binding.QueryHandle>
type SnapshotRef = External<Binding.SnapshotHandle>
type PreparedRef = External<Binding.PreparedHandle>
type ResultRef = External<Binding.ResultHandle>
type CursorRef = External<Binding.CursorHandle>
type DraftRef = External<Binding.DraftHandle>
type ChangesRef = External<Binding.ChangesHandle>
type ChangesCursorRef = External<Binding.ChangesCursorHandle>
type WitnessRef = External<Binding.WitnessHandle>

let loaded: Addon | undefined

/** The addon, loaded on first use rather than at import. */
const addon: Addon = new Proxy({} as Addon, {
	get(_target, property) {
		loaded ??= loadAddon<Addon>(process.platform, process.arch)
		return Reflect.get(loaded, property)
	},
	set(_target, property, value) {
		loaded ??= loadAddon<Addon>(process.platform, process.arch)
		return Reflect.set(loaded, property, value)
	}
})

export type {
	Addon,
	ChangesCursorRef,
	ChangesRef,
	CursorRef,
	DbRef,
	DirectoryRef,
	DraftRef,
	OperationRef,
	PreparedRef,
	QueryRef,
	ResultRef,
	RuntimeRef,
	SchemaRef,
	SnapshotRef,
	WitnessRef
}
export { addon }
