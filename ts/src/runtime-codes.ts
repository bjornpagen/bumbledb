/** The native runtime's error codes, in the order `runtimeErrorCodes()` reports them. */
export const runtimeErrorCodes = [
	"RuntimeAlreadyLive",
	"ForeignRuntime",
	"ClosedHandle",
	"HandleBusy",
	"SpentHandle",
	"QueueFull",
	"InvalidArgument",
	"Internal",
	"DirectoryBusy",
	"WriterBusy",
	"InvalidPath",
	"Io",
	"ResourceLimit",
	"Engine",
	"Cancelled"
] as const
