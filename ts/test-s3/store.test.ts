import { storeConformance } from "../test/fixtures/store-conformance.ts"
import { freshStore, target } from "./config.ts"

const store = freshStore("store")
storeConformance(`S3Store (${target})`, () => store, "conformance")
