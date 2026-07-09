//! Integrity probes — the leave/lifecycle invariants that keep HR honest under retry.

mod common;

use backbone_hr::application::service::hr_write_service::{
    HrError, HrWriteService, NewAttendance, NewEmployee, NewLeaveApplication, NewLeaveType,
};
use common::*;
use rust_decimal::Decimal;
use uuid::Uuid;

fn an_employee(company: Uuid, num: &str) -> NewEmployee {
    NewEmployee {
        company_id: company, employee_number: num.into(), user_id: None, department_id: None,
        first_name: "Budi".into(), last_name: None, designation: None, employment_type: "permanent".into(),
        date_of_joining: dt("2026-01-06T00:00:00Z"), nik: None, npwp: None, tax_status: "tk0".into(),
        bank_account_no: None, base_salary: dec("8000000"),
    }
}
async fn setup(svc: &HrWriteService, company: Uuid, quota: &str) -> (Uuid, Uuid) {
    let org = FakeOrg { company_id: company };
    let emp = svc.onboard_employee(an_employee(company, "EMP-1"), &org, &LoggingSink).await.unwrap();
    let lt = svc.create_leave_type(NewLeaveType {
        company_id: company, name: "Annual".into(), is_paid: true, annual_quota_days: dec(quota), allow_carry_forward: false,
    }).await.unwrap();
    svc.allocate_leave(company, emp, lt, 2026, dec(quota)).await.unwrap();
    (emp, lt)
}
async fn used(pool: &sqlx::PgPool, emp: Uuid, lt: Uuid) -> Decimal {
    sqlx::query_scalar("SELECT used FROM hr.leave_balances WHERE employee_id=$1 AND leave_type_id=$2 AND year=2026")
        .bind(emp).bind(lt).fetch_one(pool).await.unwrap()
}
async fn apply(svc: &HrWriteService, emp: Uuid, lt: Uuid, from: &str, to: &str) -> Uuid {
    svc.apply_leave(NewLeaveApplication { employee_id: emp, leave_type_id: lt,
        from_date: dt(from), to_date: dt(to), reason: None }).await.unwrap()
}

/// IP-1 — an approved leave is terminal: it cannot be re-approved (no double draw).
#[tokio::test]
async fn ip1_approve_once() {
    let pool = pool().await;
    let svc = HrWriteService::new(pool.clone());
    let sink = LoggingSink;
    let company = Uuid::new_v4();
    let (emp, lt) = setup(&svc, company, "12").await;
    let app = apply(&svc, emp, lt, "2026-03-02T00:00:00Z", "2026-03-04T00:00:00Z").await;
    svc.approve_leave(app, None, dt("2026-03-01T09:00:00Z"), &sink).await.unwrap();
    let again = svc.approve_leave(app, None, dt("2026-03-01T10:00:00Z"), &sink).await;
    assert!(matches!(again, Err(HrError::InvalidState(_))), "re-approve refused");
    assert_eq!(used(&pool, emp, lt).await, dec("3.00"), "drawn exactly once");
}

/// IP-2 — total approved leave never exceeds the allocation: two applications summing beyond the quota,
/// the second is refused (the gated draw serializes).
#[tokio::test]
async fn ip2_balance_never_over_drawn() {
    let pool = pool().await;
    let svc = HrWriteService::new(pool.clone());
    let sink = LoggingSink;
    let company = Uuid::new_v4();
    let (emp, lt) = setup(&svc, company, "5").await; // 5 days allocated
    let a1 = apply(&svc, emp, lt, "2026-03-02T00:00:00Z", "2026-03-05T00:00:00Z").await; // 4 days
    let a2 = apply(&svc, emp, lt, "2026-03-09T00:00:00Z", "2026-03-10T00:00:00Z").await; // 2 days
    svc.approve_leave(a1, None, dt("2026-03-01T09:00:00Z"), &sink).await.unwrap();
    let err = svc.approve_leave(a2, None, dt("2026-03-08T09:00:00Z"), &sink).await.unwrap_err();
    assert!(matches!(err, HrError::InsufficientBalance), "4+2 > 5 → second refused");
    assert_eq!(used(&pool, emp, lt).await, dec("4.00"), "only the first drawn");
}

/// IP-3 — leave cannot be applied for on an inactive (exited) employee.
#[tokio::test]
async fn ip3_apply_requires_active_employee() {
    let svc = HrWriteService::new(pool().await);
    let sink = LoggingSink;
    let company = Uuid::new_v4();
    let (emp, lt) = setup(&svc, company, "12").await;
    svc.exit_employee(emp, false, dt("2026-06-30T00:00:00Z"), &sink).await.unwrap();
    let err = svc.apply_leave(NewLeaveApplication { employee_id: emp, leave_type_id: lt,
        from_date: dt("2026-07-02T00:00:00Z"), to_date: dt("2026-07-03T00:00:00Z"), reason: None }).await.unwrap_err();
    assert!(matches!(err, HrError::InvalidState(_)), "no leave for an inactive employee");
}

/// IP-4 — attendance is one record per (employee, date): marking twice overwrites, never duplicates.
#[tokio::test]
async fn ip4_attendance_one_per_day() {
    let pool = pool().await;
    let svc = HrWriteService::new(pool.clone());
    let company = Uuid::new_v4();
    let (emp, _lt) = setup(&svc, company, "12").await;
    let d = dt("2026-03-02T00:00:00Z");
    svc.mark_attendance(NewAttendance { employee_id: emp, attendance_date: d, status: "present".into(), working_hours: dec("8") }).await.unwrap();
    svc.mark_attendance(NewAttendance { employee_id: emp, attendance_date: d, status: "half_day".into(), working_hours: dec("4") }).await.unwrap();
    let (n, status): (i64, String) = sqlx::query_as(
        "SELECT count(*), max(status::text) FROM hr.attendances WHERE employee_id=$1 AND attendance_date=$2")
        .bind(emp).bind(d).fetch_one(&pool).await.unwrap();
    assert_eq!(n, 1, "exactly one record for the day");
    assert_eq!(status, "half_day", "the second mark overwrote the first");
}

/// IP-6 (maturity council 2026-07-08) — a leave balance can NEVER go negative. The generic PATCH surface
/// can mutate an application's `days` after approval; cancelling with an inflated `days` used to restore
/// more than was drawn, driving `used` negative and manufacturing phantom entitlement. The restore is now
/// gated (`used >= days`) and a DB CHECK backstops any other writer.
#[tokio::test]
async fn ip6_balance_cannot_go_negative() {
    let pool = pool().await;
    let svc = HrWriteService::new(pool.clone());
    let sink = LoggingSink;
    let company = Uuid::new_v4();
    let (emp, lt) = setup(&svc, company, "12").await;
    let app = apply(&svc, emp, lt, "2026-03-02T00:00:00Z", "2026-03-04T00:00:00Z").await; // 3 days
    svc.approve_leave(app, None, dt("2026-03-01T09:00:00Z"), &sink).await.unwrap();
    assert_eq!(used(&pool, emp, lt).await, dec("3.00"));

    // Tamper: the generic PATCH surface sets `days` freely — inflate it beyond what was drawn.
    sqlx::query("UPDATE hr.leave_applications SET days=12 WHERE id=$1").bind(app).execute(&pool).await.unwrap();

    // Cancelling must NOT over-restore: the gated restore refuses, nothing changes.
    let err = svc.cancel_leave(app).await.unwrap_err();
    assert!(matches!(err, HrError::InvalidState(_)), "cancel of a tampered application is refused");
    assert_eq!(used(&pool, emp, lt).await, dec("3.00"), "used unchanged — never negative");

    // Backstop: the DB CHECK rejects a direct negative write to used from ANY writer.
    let direct = sqlx::query("UPDATE hr.leave_balances SET used = -1 WHERE employee_id=$1 AND leave_type_id=$2 AND year=2026")
        .bind(emp).bind(lt).execute(&pool).await;
    assert!(direct.is_err(), "the DB CHECK rejects a negative used");
}

/// IP-5 — an exited employee is terminal: re-exit is refused.
#[tokio::test]
async fn ip5_exit_terminal() {
    let svc = HrWriteService::new(pool().await);
    let sink = LoggingSink;
    let company = Uuid::new_v4();
    let (emp, _lt) = setup(&svc, company, "12").await;
    svc.exit_employee(emp, false, dt("2026-06-30T00:00:00Z"), &sink).await.unwrap();
    let again = svc.exit_employee(emp, true, dt("2026-07-30T00:00:00Z"), &sink).await;
    assert!(matches!(again, Err(HrError::InvalidState(_))), "cannot exit an inactive employee");
}
