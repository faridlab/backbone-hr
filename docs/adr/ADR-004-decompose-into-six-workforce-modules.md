# ADR-004 — Decompose into six bounded-context modules (mirror salt-laravel)

Status: accepted (decision-maker directive; supersedes ADR-003's 2-way verdict) · 2026-08-01 · Tier 5a (People pillar; posts no GL)

## Context

[ADR-003](ADR-003-extract-backbone-employee-keep-workforce-time-unified.md) recorded the council's
verdict: a **2-way** split — extract `backbone-employee`, keep the workforce-time cluster
(attendance + leave + calendar + schedule + timesheet) **unified** as one module. The council's
load-bearing reason was a coupling invariant — the *no-double-counted-payable-days* read
(`count_uncovered_absences`, a SQL anti-join from `attendance` into `leave_applications`) — which it
argued has no clean owner across separate Postgres schemas.

The decision-maker directed a **finer** decomposition that mirrors the proven salt-laravel-employee*
packages (six packages), granting latitude to merge only where coupling truly requires it.

A probe of `backbone-payroll` — the *live* consumer — then changed the picture:

> `backbone-payroll` already reads "employee unpaid days" from `backbone-hr` through an ACL
> adapter/port (`GlPostSink`, declared in its `Cargo.toml`), **not** by reaching into HR's tables.
> That port-wrapped read *is* the cross-cutting invariant the council worried about — and it already
> lives in the consumer, behind a port.

## Decision

**Decompose into six modules, mirroring salt-laravel 1:1** (dropping the redundant `employee-`
prefix per the backbone naming convention — `backbone-pos`, `backbone-crm`, etc. are unprefixed):

| backbone module | from | bounded context |
|---|---|---|
| `backbone-employee` | salt-laravel-employee | Person master — identity, employment lifecycle, payroll identity (PTKP/BPJS/bank), dependents, education, certs (13 aggregates) |
| `backbone-attendance` | salt-laravel-employee-attendance | presence + clock events |
| `backbone-calendar` | salt-laravel-employee-calendar | org-scoped holiday / working-day reference |
| `backbone-schedule` | salt-laravel-employee-schedule | shifts / roster |
| `backbone-timeoff` | salt-laravel-employee-timeoff | leave: types, requests, balances (+ the drawdown invariant) |
| `backbone-timesheet` | salt-laravel-employee-timesheet | timesheets + approvals |

- **`backbone-hr` is decomposed and removed.** Its three sub-domains migrate: employee →
  `backbone-employee`, attendance → `backbone-attendance`, leave → `backbone-timeoff`. The interim
  "rename to `backbone-worktime`" idea is **superseded** — the crate does not survive.
- Modules communicate via **logical FKs + read ports**, zero Cargo edges, wired by the HR
  backend-service (the composer). This is the established `OrgPort` pattern at finer grain.
- The "HR application" is the composing **backend-service**, not a module.

## How the council's coupling concern is handled

The no-double-counted-payable-days invariant **does not live in `backbone-attendance` or
`backbone-timeoff`**. It lives in the **consumer** — `backbone-payroll` — which already reads it behind
the `GlPostSink` port. In the six-module world, payroll's adapter changes from
`hr.period_summary(emp, range)` to three independent company-scoped reads + arithmetic:

```
present_days  = backbone_attendance.present_days(emp, range)        // days the employee HAS attendance
working_days  = backbone_calendar.working_days(company, emp, range) // scheduled working days (excl. holidays/weekends)
paid_leave    = backbone_timeoff.paid_leave_days(emp, range)        // approved PAID leave days
unpaid_days   = working_days − present_days − paid_leave             // computed in payroll
```

**Why `present_days`, not `absences`:** the coherence council dropped the invented `AttendanceStatus`
enum (no `status='absent'`). With no stored absence flag, an *absence* is derived — a scheduled
working day with no attendance and no paid leave. So `backbone-attendance` exposes only what it owns
(presence); the composer/payroll derives absences from `working_days − present_days − paid_leave`.
This keeps attendance decoupled from schedule+calendar (no cross-module read inside the port) and is
cleaner than the old cross-table anti-join. (`timeoff` exposes `paid_leave_days` and
`unpaid_leave_days` separately if the paid/unpaid split is later needed.)

**Semantic shift caught by the payroll-repoint verify (2026-08-01):** the new formula uses *derived*
absence — a working day with no attendance record and no paid leave is unpaid. The old `backbone-hr`
semantics defaulted `absent_days = 0` when no attendance existed (absence was a *stored* `status='absent'`,
not derived). So dropping the `AttendanceStatus` enum changes the contract: **the new model requires
attendance records for present days** (else they count as unpaid). The repointed payroll-seam test must
therefore seed attendance for the days the employee was present, then `unpaid_days = working_days −
present_days − paid_leave` reproduces the old result. This is more correct for a real HRIS (untracked days
shouldn't silently be paid), but it shifts a burden onto attendance-tracking completeness.

Each port query is company-scoped, preserving the `company_scope` RLS/tenant fence the council
warned would be lost in an unfenced in-memory set-diff. This is the **contract seat's remedy**,
now justified by the finding that payroll already owns the concept behind a port.

## Residual risk (the forcing function to re-evaluate)

If a **future write-path rule** couples `backbone-attendance` and `backbone-timeoff`
transactionally — e.g. "auto-deduct leave for a no-show, atomically" — separate Postgres schemas
cannot enforce it in one transaction. At that point those two modules must either **merge** or
adopt a **saga/outbox**. Not present today (YAGNI); flagged here as the single condition that would
re-open this decision. No other pair has a transactional coupling.

## Consequences

- `backbone-hr` ceases to exist; six focused modules replace it, each matching a proven
  salt-laravel package and the team's mental model.
- `backbone-payroll` (the live consumer) is the proof the seams work: its read is already a port.
- Cost: six crates/schemas + port traits + composer wiring. Justified by the proven salt-laravel
  structure, the existing payroll consumer, and that each module is a clean bounded context.
- ADR-003's *employee-extraction* conclusion **stands**; its *keep-time-unified* conclusion is
  **superseded** by this ADR. The [council report](../council/2026-08-01-module-backbone-hr-bounded-context-cleanliness.md)
  remains valid as the reasoning trail that produced the coupling analysis this ADR resolves.

## References

- [Council 2026-08-01](../council/2026-08-01-module-backbone-hr-bounded-context-cleanliness.md) — the coupling analysis.
- [ADR-003](ADR-003-extract-backbone-employee-keep-workforce-time-unified.md) — the superseded 2-way verdict (employee extraction still valid).
- [ADR-001](ADR-001-hr-boundary-and-leave-engine.md) — `OrgPort` pattern these modules generalize.
- `backbone-payroll/Cargo.toml` — the `GlPostSink` port that already wraps the cross-cutting read.
- salt-laravel source: `frameworks/salt-laravel-employee*`.
