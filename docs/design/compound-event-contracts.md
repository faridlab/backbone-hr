# Compound-Event Contracts (cross-schema writes)

> Required by the [2026-08-01 coherence council](council/2026-08-01-module-hris-constellation-coherence.md),
> finding (c): cross-module writes (e.g. `Promotion.effective` → `employee.EmploymentHistory` +
> `payroll.CompensationChange`) span two Postgres schemas with **no distributed transaction**. Linked
> only by a nullable `reference_id`, a partial failure (role-succeeds, salary-fails) diverges silently.
> This doc specifies the contract that makes the career thread close **in state, not just on paper.**

## Pattern: transactional outbox + idempotent apply + reconciliation

Every compound write follows one shape. The **source** module owns the decision and writes an
`outbox_event` row **in the same DB transaction** as its state change. A relay (in the HR composer)
delivers each event to its **target** modules, which apply it **idempotently**. A **reconciliation**
job detects any event delivered-but-not-applied; a defined **compensation** handles unrecoverable failure.

```
 source module                  composer relay                 target module(s)
 ┌────────────┐   outbox row    ┌──────────┐    event + idem-key   ┌──────────────┐
 │ tx: state  │ ───────────────▶│  poll +  │ ────────────────────▶│ apply if not │
 │ + outbox   │  (same tx)      │  deliver │                       │  already     │
 └────────────┘                 └──────────┘                       └──────────────┘
```

## `OutboxEvent` (each source module owns its own outbox table, in its own schema)

```yaml
models:
  - name: OutboxEvent
    collection: outbox_events
    description: "Transactionally-staged cross-module event. Written in the same tx as the source state change."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"], description: "The idempotency key" }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      aggregate: { type: string, attributes: ["@required", "@max(60)"], description: "e.g. Promotion, Onboarding" }
      aggregate_id: { type: uuid, attributes: ["@required"], description: "The source entity id" }
      event_type: { type: string, attributes: ["@required", "@max(60)"], description: "e.g. promotion.effective" }
      targets: { type: json, attributes: ["@required"], description: "[{module, contract, payload}]" }
      occurred_at: { type: datetime, attributes: ["@required", "@default(now)"] }
      delivered_at: { type: datetime?, description: "Set when all targets acknowledge apply" }
      attempts: { type: integer, attributes: ["@default(0)"] }
      last_error: { type: text? }
      state: { type: OutboxState, attributes: ["@required", "@default(pending)"], description: "pending/delivered/failed/dead" }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [state, occurred_at] }
      - { type: index, fields: [aggregate, aggregate_id] }

enums:
  - { name: OutboxState, variants: [{name: pending, default: true}, delivered, failed, dead] }
```

## The contracts (per compound event)

Each compound event is defined by: **source → event_type → targets[(module, contract, idempotency,
apply, reconcile, compensate)]**.

### `promotion.effective` (source: `backbone-lifecycle.Promotion`)

- **Fires when:** `Promotion` approval is `approved` AND `effective_date` is reached.
- **Targets:**
  - `employee` → `apply_role_change` — append `EmploymentHistory` row. **Idempotency key:**
    `("promotion.role", promotion_id)`; target dedupes on `(reference_id= promotion_id, action=promotion)`.
  - `payroll` → `apply_salary_change` — append `CompensationChange` row. **Idempotency key:**
    `("promotion.salary", promotion_id)`.
- **Success = both applied.** `OutboxEvent.delivered_at` set only when both targets ack.
- **Reconcile:** job finds `outbox_events` where `state=pending AND occurred_at < now()-interval`
    and re-delivers; target applies are idempotent so re-delivery is safe.
- **Compensate:** if a target is **permanently** unappliable (e.g. payroll rejects — employee has no
  bank account / salary structure), the event goes `dead`, the Promotion is flagged
  `requires_manual_reconcile`, and a notification fires to HR — the role change is **not** silently
  left half-applied. (Reversing an already-applied role change is itself an outbox event, not a direct write.)

### `onboarding.completed` (source: `backbone-lifecycle.Onboarding`)

- **Fires when:** all mandatory `OnboardingTask`s done.
- **Targets:** `employee` (ensure `Employment.status=active`), `payroll` (enroll in salary
  structure + BPJS), `approvals` (close the request). Idempotent on `onboarding_id`.

### `offboarding.closed` (source: `backbone-lifecycle.Offboarding`)

- **Fires when:** `Offboarding.status=cleared` and `last_working_day` passed.
- **Targets:** `employee` (`Employment.status=inactive`, set `date_of_exit`), `payroll`
  (final settlement + `CompensationChange` for pesangon/leave-payout), `timeoff` (encash remaining
  balance). Compensation: if payroll settlement fails, the offboarding cannot silently mark the
  employee inactive — it stays `requires_manual_reconcile`.

### `recruitment.hired` (source: `backbone-recruitment.JobOffer` on accept)

- **Fires when:** offer `accepted`.
- **Targets:** `employee` (create `Employee` + `Employment` from the offer), `lifecycle`
  (start `Onboarding`). Idempotent on `offer_id`.

## Rules that make this safe

1. **Idempotency is mandatory.** Every target `apply_*` dedupes on a deterministic key derived from
   `(event_type, aggregate_id)` — never on a fresh row id. Re-delivery is always safe.
2. **The `reference_id` on receiving append-only tables** (`EmploymentHistory.reference_id`,
   `CompensationChange.reference_id`) is the **non-null** idempotency link, set to `aggregate_id`.
   (Was nullable before — make it required for rows written by events.)
3. **No target writes back to the source.** The graph stays acyclic: source → outbox → targets.
   A failure surfaces as `state=dead` + a reconcile flag, never a reverse write.
4. **Reconciliation is a first-class job**, not an afterthought — it runs on a schedule, queries
   pending outbox rows past SLA, and re-delivers. This is what makes "eventually consistent"
   operationally honest.
5. **`delivered_at` is the source of truth** for "the compound write completed," not the source
   module's own status — workflow modules must not report `effective` until their outbox event is
   `delivered`.

## What this costs / defers

- The **composer** (HR backend-service) hosts the relay + reconciliation job (~1–2 weeks when built).
- This doc specifies the *contract*; implementation lands with the composer (rollout Phase 6). The
  important part now is that **receiving tables carry the non-null idempotency `reference_id`** and
  **every source writes an outbox row in-tx** — those schema decisions are baked in at generation time.
