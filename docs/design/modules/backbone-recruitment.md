# `backbone-recruitment` — spec

> Applicant tracking / recruitment (ATS). **Greenfield** (does not exist). Owns the pre-hire funnel;
> on hire, a candidate becomes an `Employee` in `backbone-employee`.

**Reads:** `organization` (Department, Position), `employee` (hiring manager). **On hire:** emits an
event / calls a port to create the `Employee` (does not write employee directly).

---

## `index.model.yaml`

```yaml
module: recruitment
version: 2
schema: recruitment
description: "recruitment / applicant tracking (requisition → candidate → offer → hire)"

config: { database: postgresql, soft_delete: true, audit: true, default_timestamps: true, generators: { disabled: [graphql, grpc, proto] } }
external_imports: [{ module: sapiens, types: [User] }]
imports:
  - job_requisition.model.yaml
  - candidate.model.yaml
  - job_application.model.yaml
  - interview.model.yaml
  - job_offer.model.yaml
```

## Entities

| Entity | Collection | Key fields |
|---|---|---|
| `JobRequisition` | `job_requisitions` | `company_id`, `department_id?`, `position_id?`, `title`, `headcount`, `employment_type?`, `status` (draft/open/closed/cancelled), `opened_by` (employee FK), `budget?`, `deadline?` |
| `Candidate` | `candidates` | `company_id`, `first_name`, `last_name?`, `email?`, `phone?`, `source?` (referral/job-board/...), `current_employer?`, `resume_url?` |
| `JobApplication` | `job_applications` | `company_id`, `candidate_id`, `requisition_id`, `status` (applied/screening/interview/offer/hired/rejected), `applied_at` |
| `Interview` | `interviews` | `company_id`, `application_id`, `interviewer_id` (employee FK), `scheduled_at`, `round?`, `format?` (onsite/video/phone), `rating?`, `feedback?`, `status` (scheduled/completed/cancelled) |
| `JobOffer` | `job_offers` | `company_id`, `application_id`, `proposed_salary?`, `employment_type?`, `status` (draft/extended/accepted/declined/withdrawn), `offered_at?`, `accepted_at?` |

## Enums

```yaml
enums:
  - { name: RequisitionStatus,   variants: [draft, open, on_hold, closed, cancelled] }
  - { name: ApplicationStatus,   variants: [applied, screening, interview, offer, hired, rejected] }
  - { name: InterviewStatus,     variants: [scheduled, completed, cancelled, no_show] }
  - { name: OfferStatus,         variants: [draft, extended, accepted, declined, withdrawn] }
  - { name: CandidateSource,     variants: [referral, job_board, direct, agency, walk_in] }
```

## Ports / integration

```rust
#[async_trait]
pub trait RecruitmentPort: Send + Sync {
    /// On offer acceptance — signals the HR composer to create the Employee (lifecycle/onboarding kicks in).
    async fn hire(&self, offer_id: Uuid) -> Result<HireEvent, RecruitmentRejected>;
}
```

## Notes

- **Candidate ≠ Employee.** A candidate is owned here until hired; on hire, `backbone-employee`
  creates the `Employee` and `Employment` aggregates (the recruitment → employee handoff is an event,
  not a direct write — no Cargo edge).
- Requisition references `position_id`/`department_id` (org) and `opened_by`/`interviewer_id` (employee)
  as logical FKs.
