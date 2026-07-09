//! The onboarding seam, end-to-end: **backbone-hr verifies a department against the REAL
//! backbone-organization**.
//!   HRSEAM-1 (Employee → real Department): onboarding an employee with a `department_id` resolves it
//!            through the REAL organization write/read path — it must exist AND belong to the employee's
//!            company; a foreign-company department is refused.
//! This edge is a dev-dependency ONLY — the shipped HR library has zero normal Cargo edge to organization.

mod common;

use backbone_hr::application::service::hr_write_service::{HrError, HrWriteService, NewEmployee};
use common::*;
use uuid::Uuid;

use backbone_organization::application::service::onboarding_service::{OnboardRequest, OnboardingService};
use backbone_organization::application::service::org_write_service::{NewDepartment, OrgWriteService};

fn emp_in(company: Uuid, num: &str, dept: Option<Uuid>) -> NewEmployee {
    NewEmployee {
        company_id: company, employee_number: num.into(), user_id: None, department_id: dept,
        first_name: "Sri".into(), last_name: Some("Wahyuni".into()), designation: Some("Staff".into()),
        employment_type: "permanent".into(), date_of_joining: dt("2026-01-06T00:00:00Z"),
        nik: None, npwp: None, tax_status: "tk0".into(), bank_account_no: None, base_salary: dec("6000000"),
    }
}
fn uq(p: &str) -> String { format!("{p}-{}", &Uuid::new_v4().simple().to_string()[..8]) }
/// A unique 15-digit NPWP (the org onboarding has a UNIQUE index on it — a fixed value collides across
/// test runs). Digits derived from a fresh UUID.
fn npwp() -> String {
    let u = Uuid::new_v4().as_u128();
    format!("{:015}", u % 1_000_000_000_000_000)
}

/// HRSEAM-1 — onboard against a REAL organization company + department; reject a foreign department.
#[tokio::test]
async fn hrseam1_onboard_links_a_real_department() {
    let pool = pool().await;
    let hr = HrWriteService::new(pool.clone());
    let onboarding = OnboardingService::new(pool.clone());
    let org_writer = OrgWriteService::new(pool.clone());
    let org = RealOrg { pool: pool.clone() };
    let sink = CapturingSink::new();

    // A REAL company (+ head office) and a REAL department in it.
    let mut req = OnboardRequest::new(&uq("CO"), "PT Maju Jaya");
    req.npwp = Some(npwp());
    let company = onboarding.onboard(req).await.unwrap().company_id;
    let dept = org_writer.create_department(NewDepartment {
        company_id: company, code: uq("DEPT"), name: "Finance".into(), parent_id: None,
        branch_id: None, is_group: false, manager_id: None,
    }).await.unwrap();

    // Onboard an employee into that real department — the seam resolves + verifies it.
    let emp = hr.onboard_employee(emp_in(company, "EMP-1", Some(dept)), &org, &sink).await.unwrap();
    let (dept_id, status): (Option<Uuid>, String) = sqlx::query_as(
        "SELECT department_id, status::text FROM hr.employees WHERE id=$1").bind(emp).fetch_one(&pool).await.unwrap();
    assert_eq!(dept_id, Some(dept), "employee links the real department");
    assert_eq!(status, "active");

    // A department in a DIFFERENT company is refused.
    let mut req2 = OnboardRequest::new(&uq("CO"), "PT Lain");
    req2.npwp = Some(npwp());
    let other_company = onboarding.onboard(req2).await.unwrap().company_id;
    let other_dept = org_writer.create_department(NewDepartment {
        company_id: other_company, code: uq("DEPT"), name: "HR".into(), parent_id: None,
        branch_id: None, is_group: false, manager_id: None,
    }).await.unwrap();
    let err = hr.onboard_employee(emp_in(company, "EMP-2", Some(other_dept)), &org, &sink).await.unwrap_err();
    assert!(matches!(err, HrError::Invalid(_)), "a department from another company is refused");

    // A non-existent department is refused (org rejection).
    let ghost = hr.onboard_employee(emp_in(company, "EMP-3", Some(Uuid::new_v4())), &org, &sink).await.unwrap_err();
    assert!(matches!(ghost, HrError::OrgRejected(_)), "an unknown department is refused");
}
