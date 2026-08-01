# ADR-005 — HRIS coherence fixes: Approvals context, UU PDP fence, compound-event contracts

Status: accepted · 2026-08-01 · Tier 5a (People pillar; posts no GL)

> Records the three corrective moves from the [2026-08-01 coherence council]
> (../council/2026-08-01-module-hris-constellation-coherence.md). That council's verdict: the 11 modules
> are a coherent **domain library**, not a complete or deployable **HRIS** — three concrete gaps, each
> confirmed by a cheap probe, must be closed.

## Context

The coherence council (9 seats, including 3 HR business experts) found that "module-layer completeness"
(steelman assumption D7) is the load-bearing error. Three probes settled it:
- **(a)** grep `consent|retention|pdp` across all specs = **0 hits** → `backbone-employee` stores the
  full UU PDP Article-16 PII set with no privacy scaffolding.
- **(b)** there is **no `ApprovalRequest` aggregate** → approval state is a `status` enum +
  `approved_by` embedded in 5+ entities (Promotion, Onboarding, OnboardingTask, Offboarding,
  ClearanceItem, TimeoffRequest, …) — an aggregate scattered across the constellation with no home.
- **(c)** `Promotion.effective` writes 2 append-only tables in 2 schemas linked only by a nullable
  `reference_id` → partial failure (role-succeeds, salary-fails) diverges **silently**, with no
  outbox, tx, idempotency, or compensation.

## Decision

Three fixes, each closing one probe:

1. **Extract `backbone-approvals` — the 12th module.** Spec: [`modules/backbone-approvals.md`](../design/modules/backbone-approvals.md).
   Owns `ApprovalRequest` / `ApprovalStep` / `ApprovalPolicy` / `Delegation` (state machine, multi-step
   chains, delegation, SLA, escalation). The workflow modules (lifecycle, timeoff, timesheet,
   recruitment, performance) **drop their embedded `status`/`approved_by`** and hold an
   `approval_request_id?` instead; their status becomes a projection of the approval + domain rule.
   ESS/MSS "my approvals" inbox is the read-side of this module.

2. **Fence `backbone-employee` for UU PDP.** Add `DataConsent` (lawful basis + consent + retention),
   `DataSubjectRequest` (DSAR — access/rectify/erase/export/object), and `PiiAccessLog` (append-only
   access audit) to its schema. **`backbone-employee` is marked PDP-non-deployable** until consent is
   captured for the regulated categories. May extract to a `backbone-privacy` context as it scales to
   payroll/health PII.

3. **Adopt the transactional-outbox + idempotent-apply + reconciliation pattern** for all cross-schema
   writes. Spec: [`design/compound-event-contracts.md`](../design/compound-event-contracts.md). Every
   source writes an `OutboxEvent` row **in the same tx** as its state change; targets apply
   **idempotently** (keyed on `(event_type, aggregate_id)`); the receiving `reference_id` becomes a
   **non-null** idempotency link; a reconciliation job re-delivers pending events; unrecoverable
   failures go `dead` + flag the source for manual reconcile (never a silent half-apply).

## Consequences

- The constellation grows from 11 → **12 modules** (approvals), and gains a privacy fence + a reliable
  event spine. The "career thread closes" claim now holds **in state**, not just on paper.
- Workflow modules lose their bespoke approval enums — a one-time spec refactor, cheap now (all zero-code).
- The composer (HR backend-service) inherits the relay + reconciliation job (~1–2 weeks at build time);
  the *contracts* are fixed here so implementation is mechanical.
- `backbone-employee` cannot ship to an Indonesian production tenant until the PDP fence is implemented
  and a DPO signs off on retention periods — a deliberate, explicit gate.
- YAGNI still applies to *building*: these are spec-level fixes; the build order (core 4 first) is
  unchanged (see [rollout plan](../design/rollout-plan.md)).

## What this does NOT close (parking lot, per the council)

- Statutory exports (DPK / BPJS monthly / PPh 21 SPT) — completeness-of-reporting, separate effort.
- PKWT (UU 6/2023) contract-expiry enforcement; THR ≤7-day timing rule — statutory-rule gaps inside
  existing modules.
- Succession planning + salary banding (→ `backbone-organization`); performance↔learning gap-closure.
- ESS/MSS UI surface itself (Phase C application, reads the approvals + employee contexts).

## References

- [Council 2026-08-01 — coherence](../council/2026-08-01-module-hris-constellation-coherence.md) — the verdict + probes.
- [`modules/backbone-approvals.md`](../design/modules/backbone-approvals.md) — the 12th module.
- [`design/compound-event-contracts.md`](../design/compound-event-contracts.md) — the outbox/idempotency/reconciliation contract.
- [`modules/backbone-employee.md`](../design/modules/backbone-employee.md#uu-pdp-compliance-fence) — the PDP fence.
- [ADR-004](ADR-004-decompose-into-six-workforce-modules.md) — the decomposition this sits on top of.
