//! HR's inbound identity port (hand-authored, user-owned) — the seam where onboarding validates the
//! employee's department against the REAL backbone-organization. HR holds only this trait + its DTOs;
//! a composing service wires the real organization behind it. **Zero normal Cargo edge** to organization
//! — the DTOs are the wire contract, duplicated per consumer by design. HR reads; it drives no writes.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A resolved department, read from backbone-organization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DepartmentRef {
    pub department_id: Uuid,
    pub company_id: Uuid,
    pub name: String,
}

/// The organization seam — a composing service implements it over backbone-organization. `resolve_
/// department` returns the department (so onboarding can verify it exists AND belongs to the employee's
/// company), or a rejection if it does not exist.
#[async_trait::async_trait]
pub trait OrgPort: Send + Sync {
    async fn resolve_department(&self, department_id: Uuid) -> Result<DepartmentRef, HrRejected>;
}

/// A rejection surfaced to HR from the organization seam.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HrRejected {
    pub code: String,
    pub message: String,
}
