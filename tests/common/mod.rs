//! Shared test helpers: a live pool + a fake org port (golden/integrity) + the REAL backbone-organization
//! adapter (the onboarding seam) + an event-capturing sink. Fixed timestamps via `dt()`.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use backbone_hr::application::service::hr_events::{HrEvent, HrEventSink};
pub use backbone_hr::application::service::hr_events::LoggingSink;
use backbone_hr::application::service::hr_ports::{DepartmentRef, HrRejected, OrgPort};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

pub fn dburl() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5433/backbone_hr".into())
}
pub async fn pool() -> PgPool {
    PgPool::connect(&dburl()).await.expect("connect")
}
pub fn dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}
pub fn dec(s: &str) -> Decimal {
    s.parse().unwrap()
}

/// Records every published HR event.
#[derive(Clone, Default)]
pub struct CapturingSink {
    pub events: Arc<Mutex<Vec<HrEvent>>>,
}
impl CapturingSink {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn events(&self) -> Vec<HrEvent> {
        self.events.lock().unwrap().clone()
    }
}
impl HrEventSink for CapturingSink {
    fn publish(&self, event: &HrEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}

/// A fake org port: resolves any department id to a `DepartmentRef` in a fixed company (golden/integrity).
pub struct FakeOrg {
    pub company_id: Uuid,
}
#[async_trait::async_trait]
impl OrgPort for FakeOrg {
    async fn resolve_department(&self, department_id: Uuid) -> Result<DepartmentRef, HrRejected> {
        Ok(DepartmentRef { department_id, company_id: self.company_id, name: "Fake Dept".into() })
    }
}

/// ACL over the REAL backbone-organization: resolve a department from `organization.departments`.
pub struct RealOrg {
    pub pool: PgPool,
}
#[async_trait::async_trait]
impl OrgPort for RealOrg {
    async fn resolve_department(&self, department_id: Uuid) -> Result<DepartmentRef, HrRejected> {
        let row: Option<(Uuid, String)> = sqlx::query_as(
            "SELECT company_id, name FROM organization.departments WHERE id=$1 AND (metadata->>'deleted_at') IS NULL")
            .bind(department_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| HrRejected { code: "org_lookup_failed".into(), message: e.to_string() })?;
        match row {
            Some((company_id, name)) => Ok(DepartmentRef { department_id, company_id, name }),
            None => Err(HrRejected { code: "department_not_found".into(), message: format!("no department {department_id}") }),
        }
    }
}
