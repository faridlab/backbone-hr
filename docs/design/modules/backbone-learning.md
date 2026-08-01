# `backbone-learning` — spec

> Learning & development: training catalog, enrollments, competency matrix. **Greenfield.**

**Reads:** `employee` (Employee). On course completion, signals `backbone-employee` to record the
earned certification (employee owns *held* certs; learning owns the *catalog* + *enrollments*).

---

## `index.model.yaml`

```yaml
module: learning
version: 2
schema: learning
description: "learning & development — courses, enrollments, competencies"

config: { database: postgresql, soft_delete: true, audit: true, default_timestamps: true, generators: { disabled: [graphql, grpc, proto] } }
external_imports: [{ module: sapiens, types: [User] }]
imports:
  - course.model.yaml
  - course_enrollment.model.yaml
  - competency.model.yaml
  - employee_competency.model.yaml
  - skill.model.yaml
  - employee_skill.model.yaml
```

## Entities

| Entity | Collection | Key fields |
|---|---|---|
| `Course` | `courses` | `company_id`, `name`, `description?`, `format` (online/onsite/blended), `duration_hours?`, `cost?`, `provider?`, `certification_id?` (if completion grants a cert — logical FK to a cert catalog), `is_active` |
| `CourseEnrollment` | `course_enrollments` | `company_id`, `course_id`, `employee_id`, `status` (enrolled/in_progress/completed/withdrawn), `enrolled_at`, `completed_at?`, `score?` |
| `Competency` | `competencies` | `company_id`, `name`, `category?` (e.g. technical/leadership/safety), `description?` |
| `EmployeeCompetency` | `employee_competencies` | `company_id`, `employee_id`, `competency_id`, `level` (1–5), `assessed_at?` |
| `Skill` | `skills` | `company_id`, `name`, `category?` (technical/soft/tool/language), `description?` |
| `EmployeeSkill` | `employee_skills` | `company_id`, `employee_id`, `skill_id`, `proficiency` (novice→expert), `years_experience?`, `verification` (self/assessed/certified), `verified_by?` |

## Enums

```yaml
enums:
  - { name: CourseFormat,        variants: [online, onsite, blended, self_paced] }
  - { name: EnrollmentStatus,    variants: [enrolled, in_progress, completed, withdrawn, failed] }
  - { name: CompetencyCategory,  variants: [technical, leadership, soft_skill, safety, compliance] }
```

## `Skill` / `EmployeeSkill` — granular capability (distinct from Competency)

> **Skill ≠ Competency.** Skills are granular proficiency tags for staffing/matching/gap-analysis
> (Python, React, Negotiation); Competencies are the appraisal-framework capabilities assessed in
> reviews (Leadership, Communication). Both live in the capability domain but stay separate tables.

```yaml
models:
  - name: Skill
    collection: skills
    description: "Granular skill in the catalog (technical / soft / tool / language)."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK organization.Company.id" }
      name: { type: string, attributes: ["@required", "@max(120)"] }
      category: { type: SkillCategory?, description: "technical/soft/tool/language" }
      description: { type: text? }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: unique, fields: [company_id, name], where: "deleted_at IS NULL" }

  - name: EmployeeSkill
    collection: employee_skills
    description: "An employee's proficiency in a skill."
    fields:
      id: { type: uuid, attributes: ["@id", "@default(uuid)"] }
      company_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"] }
      employee_id: { type: uuid, attributes: ["@required", "@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id" }
      skill_id: { type: uuid, attributes: ["@required", "@foreign_key(Skill.id)"], description: "# FK skills.id (in-module)" }
      proficiency: { type: ProficiencyLevel, attributes: ["@required", "@default(intermediate)"] }
      years_experience: { type: decimal?, attributes: ["@precision(5,1)", "@non_negative"] }
      verification: { type: SkillVerification, attributes: ["@required", "@default(self_reported)"] }
      verified_by: { type: uuid?, attributes: ["@exclude_from_foreign_key_check"], description: "# logical FK employee.Employee.id (assessor)" }
      last_used_at: { type: date? }
      metadata: { type: Metadata, attributes: ["@audit_metadata"] }
    indexes:
      - { type: index, fields: [employee_id] }
      - { type: index, fields: [skill_id] }

enums:
  - { name: SkillCategory, variants: [technical, soft, tool, language, domain, certification] }
  - name: ProficiencyLevel
    description: "1 (novice) → 5 (expert)"
    variants:
      - { name: novice, description: "1 — aware" }
      - { name: beginner, description: "2 — basic application" }
      - { name: intermediate, default: true, description: "3 — independent" }
      - { name: advanced, description: "4 — deep / can guide" }
      - { name: expert, description: "5 — authority" }
  - { name: SkillVerification, variants: [{name: self_reported, default: true}, assessed, certified] }
```

## Ports

```rust
#[async_trait]
pub trait LearningPort: Send + Sync {
    async fn training_history(&self, company_id: Uuid, employee_id: Uuid) -> Result<Vec<EnrollmentSummary>, LearningRejected>;
    async fn competency_matrix(&self, company_id: Uuid, employee_id: Uuid) -> Result<Vec<EmployeeCompetency>, LearningRejected>;
    /// Granular skills (for staffing / matching / gap-analysis) — distinct from the competency matrix.
    async fn skills(&self, company_id: Uuid, employee_id: Uuid) -> Result<Vec<EmployeeSkill>, LearningRejected>;
}
```

## Notes

- **Certification ownership split:** `backbone-employee` owns `EmployeeCertification` (the certs a
  person *holds*, incl. expiry for compliance). `backbone-learning` owns the `Course` catalog +
  enrollments. On completion of a cert-granting course, learning emits an event → employee records
  the held cert. Keeps each context cohesive.
- Competency level (1–5) can feed `backbone-performance` talent calibration — cross-read, no write.
