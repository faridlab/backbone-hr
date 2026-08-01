# ADR-002 — HRIS scope: backbone-hr as a module in a constellation

Status: proposed · 2026-08-01 · Tier 5a (People pillar; posts no GL)

> **Module boundaries now governed by [ADR-004](ADR-004-decompose-into-six-workforce-modules.md)**
> (2026-08-01). The constellation thesis below **stands** — the HRIS is many modules, not one — but the
> decomposition is **six modules** mirroring salt-laravel (`backbone-employee`, `-attendance`,
> `-calendar`, `-schedule`, `-timeoff`, `-timesheet`); `backbone-hr` is decomposed and removed.
> ADR-003's 2-way council verdict was superseded by a decision-maker directive; ADR-004 records why the
> council's coupling concern is resolved (the cross-cutting read already lives behind
> `backbone-payroll`'s port). The roadmap below is illustrative only — ADR-004 is authoritative.

## Context

[ADR-001](ADR-001-hr-boundary-and-leave-engine.md) shipped `backbone-hr` as the **people master +
leave/attendance inputs** — deliberately ahead of `backbone-payroll`, which has nothing to compute
without an employee roster, salary identity, and time/leave inputs. Phase-4 (0.2.0) hardened that
foundation: per-company `employee_number` is now a race-proof partial-unique DB index, year
extraction is panic-free, and year-spanning leave applications are rejected at the gate.

The open question is **how a full HRIS grows from here.** A Human Resource Information System spans
payroll, recruitment, performance, learning, benefits, and analytics — far beyond people + time +
leave. Two paths:

- **A. Monolith** — keep adding domains into `backbone-hr` until it owns everything people-adjacent.
- **B. Bounded contexts** — `backbone-hr` is one module among several; spin each new domain out as its
  own module that consumes the others through logical FKs and read ports, mirroring the tax-overlay
  split already established in ADR-001 §4.

## Decision

**Path B. The HRIS is a constellation of modules, not one.** *(End state confirmed by the
2026-08-01 council; the boundaries are refined in ADR-003.)*

1. **`backbone-hr` is the workforce-time bounded context** — attendance + leave + calendar + schedule
   + timesheet. It is a **module**, not an application/service. The people master is **extracted** to
   `backbone-employee` (ADR-003); `backbone-hr` reads `employee_id` through a port, exactly as it
   reads `organization.Department` via `OrgPort` today (ADR-001 §2). It does **not** absorb payroll,
   recruitment, performance, learning, or benefits.
2. **Each new HRIS domain is its own module**, consuming others through logical FKs and read ports —
   no normal Cargo edge, exactly as modules read `organization` / `sapiens` today (ADR-001 §2).
3. **Sequencing follows what unblocks the next consumer** (Roadmap below). `backbone-employee` is
   first (the master every consumer reads, and a live PTKP/dependents correctness fix); then payroll,
   because HR was built for it (ADR-001, Context).
4. **The Phase-A completion items are the parked gates from ADR-001**, promoted into sequenced work —
   not reinvented here. They now complete the **workforce-time** context.

## Consequences

- `backbone-hr` stays small and regen-safe; payroll math, recruitment pipelines, and appraisal
  cycles never pollute its schema or `module.rs`.
- A consumer (e.g. payroll) can be built, versioned, and deployed independently; turning the time
  module off still changes no ledger (ADR-001 §5).
- Cost: more modules to orchestrate — the composing service wires more ports. Acceptable: that *is*
  the backbone module model.
- The time cluster is **intentionally not split** into calendar/schedule/timeoff/timesheet/attendance
  peers — the council proved those are one context (see ADR-003). Splitting is deferred to a forcing
  function.

## Roadmap

### Phase 0 — extract the people master *(ADR-003)*

New peer module **`backbone-employee`**: the people master as real aggregates (mirroring
salt-laravel's 13 tables), with PTKP tax tier **derived from dependents** instead of a free enum.
`backbone-hr` reads it via a port. Fixes a live payroll-correctness gap and is the prerequisite for
every downstream consumer.

### Phase A — complete `backbone-hr`'s workforce-time inputs *(closes ADR-001 parking-lot gates)*

| Work | Resolves (ADR-001 gate) | Why it unblocks |
|---|---|---|
| **Holiday calendar** (`holiday` entity) | "Holiday-calendar leave days" | leave `days` becomes working-days; payroll payable-days correct |
| **Shift / roster + overtime** (`shift`, `overtime`) | net-new | planned-vs-actual attendance; OT hours feed payroll |
| **Leave accrual / carry-forward job** | "Carry-forward automation" | year-end allocation rollover; proration on join |
| **Attendance regularization** | net-new | employee-initiated missed-punch correction workflow |
| **Employee document store** (`employee_document`) | net-new | contracts, KTP, NPWP, certificates (onboarding / compliance) |

**Resolved since ADR-001 (no action):** `employee_number` race → DB partial-unique index (Phase-4);
cross-year draw → reject year-spanning applications (Phase-4).

**Open hardening gate (non-blocking):** authorization review of the generic CRUD/PATCH mutation
surface on leave status/days (ADR-001 parking lot).

### Phase B — sibling modules consuming `backbone-employee` + `backbone-hr`

| Module | Reads | Owns |
|---|---|---|
| **`backbone-employee`** *(Phase 0)* | organization | the people master: identity, employment lifecycle, payroll identity (PTKP/BPJS/bank), dependents |
| **`backbone-payroll`** ⭐ next | employee identity + salary; attendance; leave `period_summary`; overtime | salary components, PPh 21, BPJS Kesehatan / TK, THR, payslips, GL posting |
| **`backbone-organization`** | — | Company, Department, **Position, Grade, Designation**, org chart (`backbone-hr` already reads its `Department` via `OrgPort`) |
| **`backbone-recruitment`** | employee (on hire) | requisitions, candidates, pipeline, offers |
| **`backbone-performance`** | employee, manager, designation | OKRs, appraisal cycles, 9-box, 360 |
| **`backbone-lifecycle`** | employee status / dates | onboarding / offboarding, clearance, 🇮🇩 pesangon final settlement |
| **`backbone-learning`** | employee, designation | training, certifications + expiry, competencies |

### Phase C — platform / app layer (the "HR application": composes modules, is not a module)

Employee & manager self-service (ESS / MSS), approval inbox, dashboards (headcount, attrition,
attendance), notifications engine, 🇮🇩 compliance exports (DP3 / SPT).

## References

- [ADR-003](ADR-003-extract-backbone-employee-keep-workforce-time-unified.md) — the decomposition decision + probe evidence (employee out, time unified, 6-way mirror rejected).
- [Council 2026-08-01 — bounded-context-cleanliness](../council/2026-08-01-module-backbone-hr-bounded-context-cleanliness.md).
- [ADR-001](ADR-001-hr-boundary-and-leave-engine.md) — HR boundary, leave-balance engine, onboarding seam.
