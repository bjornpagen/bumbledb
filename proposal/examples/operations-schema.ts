// Current bumbledb authoring syntax. Proposal-only schema example; no database
// is opened and no probability runtime API is invented here.
import {
  bool, closed, closedId, contained, f64, i64, interval,
  key, on, relation, schema, str, uuid
} from "@bjornpagen/bumbledb"

const Cause = closed("Cause", ["deploy", "database", "provider", "other"])
const Severity = closed("Severity", ["cosmetic", "degraded", "outage"])
const Action = closed("Action", ["rollback", "failover", "investigate"])

const Service = relation("Service", { id: uuid, name: str, owner: str })
const Incident = relation("Incident", {
  id: uuid, service: uuid, during: interval(i64)
})
const Report = relation("Report", {
  id: uuid, incident: uuid, origin: str, text: str
})
const Dependency = relation("Dependency", {
  id: uuid, consumer: uuid, provider: uuid, active: interval(i64)
})
const Source = relation("Source", {
  id: uuid, model: str, promptRevision: str, evaluationSlice: str
})
const Assessment = relation("Assessment", {
  id: uuid, incident: uuid, source: uuid, stateDigest: str, resolvedModel: str
})
const UsesReport = relation("UsesReport", { assessment: uuid, report: uuid, incident: uuid })
const CauseMass = relation("CauseMass", {
  assessment: uuid, cause: closedId(Cause), reportedMass: f64
})
const SeverityMass = relation("SeverityMass", {
  assessment: uuid, severity: closedId(Severity), reportedMass: f64
})
const OutageForecast = relation("OutageForecast", {
  assessment: uuid, reportedProbability: f64
})
const Audit = relation("Audit", {
  // This example audits the primary Choice classification. Other answer
  // families need their own question/source bindings and keyed audits.
  assessment: uuid, isWrong: bool, randomSample: bool
})
const ActionCost = relation("ActionCost", {
  action: closedId(Action), cause: closedId(Cause), loss: f64
})

// These are recorded source outputs. Existing f64, keys, and containment do
// not assert calibration or enforce a coherent joint probability model.
// The proposal adds that explicit model-construction/judgment stage.
export const Operations = schema("Operations", {
  Cause, Severity, Action, Service, Incident, Report, Dependency,
  Source, Assessment, UsesReport, CauseMass, SeverityMass,
  OutageForecast, Audit, ActionCost
}, [
  key(Service, ["id"]), key(Incident, ["id"]), key(Report, ["id"]),
  key(Report, ["id", "incident"]),
  key(Dependency, ["id"]), key(Source, ["id"]), key(Assessment, ["id"]),
  key(Assessment, ["id", "incident"]),
  key(UsesReport, ["assessment", "report"]),
  key(CauseMass, ["assessment", "cause"]),
  key(SeverityMass, ["assessment", "severity"]),
  key(OutageForecast, ["assessment"]), key(Audit, ["assessment"]),
  key(ActionCost, ["action", "cause"]),
  contained(on(Incident, "service"), on(Service, "id")),
  contained(on(Report, "incident"), on(Incident, "id")),
  contained(on(Dependency, "consumer"), on(Service, "id")),
  contained(on(Dependency, "provider"), on(Service, "id")),
  contained(on(Assessment, "incident"), on(Incident, "id")),
  contained(on(Assessment, "source"), on(Source, "id")),
  contained(on(UsesReport, ["assessment", "incident"]), on(Assessment, ["id", "incident"])),
  contained(on(UsesReport, ["report", "incident"]), on(Report, ["id", "incident"])),
  contained(on(CauseMass, "assessment"), on(Assessment, "id")),
  contained(on(CauseMass, "cause"), on(Cause, "id")),
  contained(on(SeverityMass, "assessment"), on(Assessment, "id")),
  contained(on(SeverityMass, "severity"), on(Severity, "id")),
  contained(on(OutageForecast, "assessment"), on(Assessment, "id")),
  contained(on(Audit, "assessment"), on(Assessment, "id")),
  contained(on(ActionCost, "action"), on(Action, "id")),
  contained(on(ActionCost, "cause"), on(Cause, "id"))
])
