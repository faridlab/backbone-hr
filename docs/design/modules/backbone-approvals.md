# `backbone-approvals` — spec

> The **12th module** — extracted by the [2026-08-01 coherence council](../council/2026-08-01-module-hris-constellation-coherence.md).
> Approval state was a `status` enum + `approved_by` hidden inside 5+ entities (Promotion, Onboarding,
> OnboardingTask, Offboarding, ClearanceItem, plus TimeoffRequest, TimesheetApproval, JobOffer,
> Appraisal). That is an aggregate scattered across the constellation with no home. This module owns it.

**The approval spine.** Any workflow module that needs a human decision creates an `ApprovalRequest`
here and gates its transition on the decision — instead of carrying its own `status`/`approved_by`.
Delegation, SLA, multi-step chains, and escalation all live in one place.

**Reads:** `employee` (requester, approvers, delegates), `organization` (role/position/department-head
resolution). **Consumed by:** lifecycle (Promotion/Onboarding/Offboarding/Clearance), timeoff,
timesheet, recruitment (Offer), performance (Appraisal).

---

## `index.model.yaml`

```yaml
module: approvals
version: 2
schema: approvals
description: "the approval spine — requests, multi-step chains, delegation, SLA"

config: { database: postgresql, soft_delete: true, audit: true, default_timestamps: true, generators: { disabled: [graphql, grpc, proto] } }
external_imports: [{ module: sapiens, types: [User] }]
imports:
  - approval_policy.model.yaml
  - approval_request.model.yaml
  - approval_step.model.yaml
  - delegation.model.yaml
```

## Entities

| Entity | Collection | Key fields |
|---|---|---|
| `ApprovalPolicy` | `approval_policies` | `company_id`, `resource_type` (promotion/onboarding/offboarding/clearance/leave/timesheet/offer/appraisal), `name`, `is_active`, `description?` | 
| `ApprovalStepTemplate` | `approval_step_templates` | `policy_id`, `step_no`, `approver_kind` (specific_employee/manager_of_requester/department_head/role/position), `approver_ref?` (uuid — the specific employee/role/position), `sla_hours?`, `all_of?` (json — quorum/consensus rules) |
| `ApprovalRequest` | `approval_requests` | `company_id`, `resource_type`, `resource_id` (the thing being approved — logical FK to the workflow module's entity), `policy_id?`, `requested_by` (employee FK), `status` (draft/pending/approved/rejected/withdrawn/cancelled), `current_step?`, `priority` (low/normal/high/urgent), `submitted_at?`, `decided_at?`, `decided_by?`, `summary?` (json — payload snapshot for the approver) |
| `ApprovalStep` | `approval_steps` | `company_id`, `request_id`, `step_no`, `approver_kind`, `assigned_to` (employee FK), `delegated_from?` (employee FK), `status` (pending/approved/rejected/delegated/skipped), `acted_at?`, `comment?`, `sla_due_at?` |
| `Delegation` | `delegations` | `company_id`, `approver_id` (employee FK), `delegate_to_id` (employee FK), `valid_from`, `valid_to`, `reason?`, `status` (active/revoked) |

**Unique invariant:** one **live** `ApprovalRequest` per `(company_id, resource_type, resource_id)` —
a workflow entity has at most one non-deleted chain. Implemented as a partial unique index
`where deleted_at IS NULL` (the DSL rejected a `status='pending'` predicate), so a withdrawn/cancelled
request must be **soft-deleted** before a new one for the same resource can be filed.

## Enums

```yaml
enums:
  - { name: ApprovalResourceType, variants: [promotion, onboarding, onboarding_task, offboarding, clearance, leave, timesheet, offer, appraisal, custom] }
  - { name: ApprovalStatus,       variants: [{name: draft, default: true}, pending, approved, rejected, withdrawn, cancelled] }
  - { name: ApprovalStepStatus,   variants: [pending, approved, rejected, delegated, skipped, escalated] }
  - { name: ApproverKind,         variants: [specific_employee, manager_of_requester, department_head, role, position] }
  - { name: ApprovalPriority,     variants: [{name: low}, {name: normal, default: true}, high, urgent] }
  - { name: DelegationStatus,      variants: [{name: active, default: true}, revoked] }
```

## Ports

```rust
#[async_trait]
pub trait ApprovalsPort: Send + Sync {
    /// A workflow module creates a request when it enters its "needs approval" state.
    /// Resolves the policy → assigns step 1 approver (honoring active delegations). Returns the request id.
    async fn request(&self, company_id: Uuid, resource: ApprovalResource, payload: Value) -> Result<Uuid, ApprovalsRejected>;
    /// The current decision state — workflow modules poll/subscribe to gate their transitions.
    async fn status(&self, request_id: Uuid) -> Result<ApprovalStatus, ApprovalsRejected>;
    /// An approver (or their delegate) decides the current step → advances/rejects the chain.
    async fn decide(&self, request_id: Uuid, step_id: Uuid, decision: Decision) -> Result<ApprovalOutcome, ApprovalsRejected>;
}

// ApprovalOutcome := Approved (all steps done) | Advanced (next step) | Rejected | Withdrawn
```

## How the workflow modules re-point (the fix)

Each module **drops its `status`/`approved_by` approval fields** and holds an `approval_request_id?`
instead. Its own `status` becomes a *projection* of the approval + domain rule:

| Module / entity | Before | After |
|---|---|---|
| `lifecycle.Promotion` | `status` enum + `approved_by` | `approval_request_id?`; **effective** = request approved AND `effective_date` reached (composer applies role+salary events) |
| `lifecycle.Onboarding` / `OnboardingTask` / `Offboarding` / `ClearanceItem` |各自的 `status` | `approval_request_id?` where a sign-off is needed |
| `timeoff.TimeoffRequest` | `status` (pending/approved/…) + `approval_employee_id` | `approval_request_id?`; the drawdown fires when request → approved |
| `timesheet.TimesheetApproval` | own `status` + `approver_id` | collapse into an `ApprovalRequest` (`resource_type=timesheet`) |
| `recruitment.JobOffer` | `status` (extended/accepted/…) | acceptance is a decision; `approval_request_id?` for internal offer sign-off |
| `performance.Appraisal` | `status` incl. calibrated | review sign-off + calibration = approval steps |

A workflow keeps a *domain* status only where it is NOT an approval (e.g. `Promotion` still needs
`effective_date` semantics; `Onboarding` still tracks task completion) — the approval decision itself
moves here.

## Notes

- **ESS/MSS inbox is the read-side of this module** (Phase C app) — "my pending approvals" = query
  `ApprovalStep` where `assigned_to = me` and `status = pending`. That surface is app composition; the
  *data* it reads lives here.
- **Delegation** is first-class: an approver on leave delegates; `ApprovalStep.assigned_to` resolves
  via active `Delegation` at decision time.
- **SLA / escalation:** `sla_due_at` per step enables "overdue → escalate to manager_of_requester" as
  a background job, without each module reimplementing it.
