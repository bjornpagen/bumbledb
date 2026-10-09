import { hostedConformance } from "../test/fixtures/hosted-conformance.ts"
import { freshStore, target } from "./config.ts"

hostedConformance(`S3Store (${target})`, () => freshStore("database"))
