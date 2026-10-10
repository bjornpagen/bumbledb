/** Alchemy's DocumentDB types import these from mongodb, which this app never installs or uses. */
declare module "mongodb" {
	export type Db = unknown
	export type MongoClient = unknown
	export type MongoClientOptions = unknown
}
