//! Golden cases — the leave-balance oracle + the employee lifecycle. Approving leave draws the balance
//! down (gated on availability); cancelling an approved leave restores it.

mod common;

use backbone_hr::application::service::hr_events::HrEvent;
use backbone_hr::application::service::hr_write_service::{
    HrError, HrWriteService, NewAttendance, NewEmployee, NewLeaveApplication, NewLeaveType,
};
use common::*;
use rust_decimal::Decimal;
use uuid::Uuid;

fn an_employee(company: Uuid, num: &str) -> NewEmployee {
    NewEmployee {
        company_id: company, employee_number: num.into(), user_id: None, department_id: None,
        first_name: "Budi".into(), last_name: Some("Santoso".into()), designation: None,
        employment_type: "permanent".into(), date_of_joining: dt("2026-01-06T00:00:00Z"),
        nik: Some("3201010101010001".into()), npwp: None, tax_status: "k1".into(),
        bank_account_no: None, base_salary: dec("8000000"),
    }
}
async fn balance(pool: &sqlx::PgPool, emp: Uuid, lt: Uuid, year: i32) -> (Decimal, Decimal) {
    sqlx::query_as("SELECT allocated, used FROM hr.leave_balances WHERE employee_id=$1 AND leave_type_id=$2 AND year=$3")
        .bind(emp).bind(lt).bind(year).fetch_one(pool).await.unwrap()
}

/// HGC-1 — apply + approve draws the balance down by the exact day count; emits LeaveApproved.
#[tokio::test]
async fn hgc1_approve_draws_balance() {
    let pool = pool().await;
    let svc = HrWriteService::new(pool.clone());
    let sink = CapturingSink::new();
    let company = Uuid::new_v4();
    let org = FakeOrg { company_id: company };

    let emp = svc.onboard_employee(an_employee(company, "EMP-1"), &org, &sink).await.unwrap();
    let annual = svc.create_leave_type(NewLeaveType {
        company_id: company, name: "Cuti Tahunan".into(), is_paid: true, annual_quota_days: dec("12"),
        allow_carry_forward: false,
    }).await.unwrap();
    svc.allocate_leave(company, emp, annual, 2026, dec("12")).await.unwrap();

    // 3-day leave (Mon–Wed inclusive).
    let app = svc.apply_leave(NewLeaveApplication {
        employee_id: emp, leave_type_id: annual, from_date: dt("2026-03-02T00:00:00Z"),
        to_date: dt("2026-03-04T00:00:00Z"), reason: None,
    }).await.unwrap();
    let days: Decimal = sqlx::query_scalar("SELECT days FROM hr.leave_applications WHERE id=$1").bind(app).fetch_one(&pool).await.unwrap();
    assert_eq!(days, dec("3"), "inclusive 3-day span");

    svc.approve_leave(app, Some(Uuid::new_v4()), dt("2026-03-01T09:00:00Z"), &sink).await.unwrap();
    assert_eq!(balance(&pool, emp, annual, 2026).await, (dec("12.00"), dec("3.00")), "3 of 12 used");
    let status: String = sqlx::query_scalar("SELECT status::text FROM hr.leave_applications WHERE id=$1").bind(app).fetch_one(&pool).await.unwrap();
    assert_eq!(status, "approved");
    assert!(sink.events().iter().any(|e| matches!(e, HrEvent::LeaveApproved(l) if l.days == dec("3"))));
}

/// HGC-2 — approving beyond the balance is refused, and nothing is drawn (fail-closed).
#[tokio::test]
async fn hgc2_approve_insufficient_refused() {
    let pool = pool().await;
    let svc = HrWriteService::new(pool.clone());
    let sink = LoggingSink;
    let company = Uuid::new_v4();
    let org = FakeOrg { company_id: company };
    let emp = svc.onboard_employee(an_employee(company, "EMP-1"), &org, &sink).await.unwrap();
    let annual = svc.create_leave_type(NewLeaveType {
        company_id: company, name: "Annual".into(), is_paid: true, annual_quota_days: dec("2"), allow_carry_forward: false,
    }).await.unwrap();
    svc.allocate_leave(company, emp, annual, 2026, dec("2")).await.unwrap();

    let app = svc.apply_leave(NewLeaveApplication {
        employee_id: emp, leave_type_id: annual, from_date: dt("2026-03-02T00:00:00Z"),
        to_date: dt("2026-03-06T00:00:00Z"), reason: None, // 5 days > 2 allocated
    }).await.unwrap();
    let err = svc.approve_leave(app, None, dt("2026-03-01T09:00:00Z"), &sink).await.unwrap_err();
    assert!(matches!(err, HrError::InsufficientBalance));
    assert_eq!(balance(&pool, emp, annual, 2026).await, (dec("2.00"), dec("0.00")), "nothing drawn");
    let status: String = sqlx::query_scalar("SELECT status::text FROM hr.leave_applications WHERE id=$1").bind(app).fetch_one(&pool).await.unwrap();
    assert_eq!(status, "pending", "application stays pending — the transition rolled back with the draw");
}

/// HGC-3 — cancelling an APPROVED leave restores the balance.
#[tokio::test]
async fn hgc3_cancel_restores_balance() {
    let pool = pool().await;
    let svc = HrWriteService::new(pool.clone());
    let sink = LoggingSink;
    let company = Uuid::new_v4();
    let org = FakeOrg { company_id: company };
    let emp = svc.onboard_employee(an_employee(company, "EMP-1"), &org, &sink).await.unwrap();
    let annual = svc.create_leave_type(NewLeaveType {
        company_id: company, name: "Annual".into(), is_paid: true, annual_quota_days: dec("12"), allow_carry_forward: false,
    }).await.unwrap();
    svc.allocate_leave(company, emp, annual, 2026, dec("12")).await.unwrap();
    let app = svc.apply_leave(NewLeaveApplication {
        employee_id: emp, leave_type_id: annual, from_date: dt("2026-03-02T00:00:00Z"),
        to_date: dt("2026-03-04T00:00:00Z"), reason: None,
    }).await.unwrap();
    svc.approve_leave(app, None, dt("2026-03-01T09:00:00Z"), &sink).await.unwrap();
    assert_eq!(balance(&pool, emp, annual, 2026).await.1, dec("3.00"));

    svc.cancel_leave(app).await.unwrap();
    assert_eq!(balance(&pool, emp, annual, 2026).await, (dec("12.00"), dec("0.00")), "balance restored");
    let status: String = sqlx::query_scalar("SELECT status::text FROM hr.leave_applications WHERE id=$1").bind(app).fetch_one(&pool).await.unwrap();
    assert_eq!(status, "cancelled");
}

/// HGC-5 (completeness council 2026-07-08) — the payroll-facing OUTPUT. `period_summary` reconciles
/// approved leave (split paid/unpaid, clamped to the period) and attendance absences (uncovered by
/// leave, so no double-count) into the days that scale pay; `LeaveApproved` carries `is_paid` so payroll
/// never guesses. Without this, payroll — HR's whole reason to exist — has no clean input.
#[tokio::test]
async fn hgc5_payroll_facing_period_summary() {
    let pool = pool().await;
    let svc = HrWriteService::new(pool.clone());
    let sink = CapturingSink::new();
    let company = Uuid::new_v4();
    let org = FakeOrg { company_id: company };
    let emp = svc.onboard_employee(an_employee(company, "EMP-1"), &org, &sink).await.unwrap();

    let paid = svc.create_leave_type(NewLeaveType {
        company_id: company, name: "Cuti Tahunan".into(), is_paid: true, annual_quota_days: dec("12"), allow_carry_forward: false,
    }).await.unwrap();
    let unpaid = svc.create_leave_type(NewLeaveType {
        company_id: company, name: "Cuti di luar tanggungan".into(), is_paid: false, annual_quota_days: dec("30"), allow_carry_forward: false,
    }).await.unwrap();
    svc.allocate_leave(company, emp, paid, 2026, dec("12")).await.unwrap();
    svc.allocate_leave(company, emp, unpaid, 2026, dec("30")).await.unwrap();

    // A 2-day PAID leave and a 3-day UNPAID leave in March, plus one absent day (not on leave).
    let pa = svc.apply_leave(NewLeaveApplication { employee_id: emp, leave_type_id: paid,
        from_date: dt("2026-03-02T00:00:00Z"), to_date: dt("2026-03-03T00:00:00Z"), reason: None }).await.unwrap();
    svc.approve_leave(pa, None, dt("2026-03-01T09:00:00Z"), &sink).await.unwrap();
    let ua = svc.apply_leave(NewLeaveApplication { employee_id: emp, leave_type_id: unpaid,
        from_date: dt("2026-03-10T00:00:00Z"), to_date: dt("2026-03-12T00:00:00Z"), reason: None }).await.unwrap();
    svc.approve_leave(ua, None, dt("2026-03-09T09:00:00Z"), &sink).await.unwrap();
    svc.mark_attendance(NewAttendance { employee_id: emp, attendance_date: dt("2026-03-20T00:00:00Z"),
        status: "absent".into(), working_hours: dec("0") }).await.unwrap();

    // The payroll input for March: 2 paid leave, 3 unpaid leave, 1 uncovered absence → payroll deducts 4.
    let s = svc.period_summary(emp, dt("2026-03-01T00:00:00Z"), dt("2026-03-31T00:00:00Z")).await.unwrap();
    assert_eq!(s.paid_leave_days, dec("2"), "paid leave does not cut pay");
    assert_eq!(s.unpaid_leave_days, dec("3"), "unpaid leave cuts pay");
    assert_eq!(s.absent_days, 1, "uncovered absence cuts pay");

    // The LeaveApproved event carries is_paid so a subscriber can react in real time.
    let paids: Vec<bool> = sink.events().into_iter().filter_map(|e| match e {
        HrEvent::LeaveApproved(l) => Some(l.is_paid), _ => None }).collect();
    assert!(paids.contains(&true) && paids.contains(&false), "the paid/unpaid signal reaches payroll");
}

/// HGC-4 — the input guards.
#[tokio::test]
async fn hgc4_validation() {
    let pool = pool().await;
    let svc = HrWriteService::new(pool.clone());
    let sink = LoggingSink;
    let company = Uuid::new_v4();
    let org = FakeOrg { company_id: company };

    let no_num = svc.onboard_employee(NewEmployee { employee_number: "  ".into(), ..an_employee(company, "x") }, &org, &sink).await;
    assert!(matches!(no_num, Err(HrError::Invalid(_))), "employee needs a number");

    let emp = svc.onboard_employee(an_employee(company, "EMP-1"), &org, &sink).await.unwrap();
    let dup = svc.onboard_employee(an_employee(company, "EMP-1"), &org, &sink).await;
    assert!(matches!(dup, Err(HrError::Invalid(_))), "duplicate employee_number refused");

    let lt = svc.create_leave_type(NewLeaveType {
        company_id: company, name: "Annual".into(), is_paid: true, annual_quota_days: dec("12"), allow_carry_forward: false,
    }).await.unwrap();
    let bad_range = svc.apply_leave(NewLeaveApplication {
        employee_id: emp, leave_type_id: lt, from_date: dt("2026-03-05T00:00:00Z"),
        to_date: dt("2026-03-02T00:00:00Z"), reason: None,
    }).await;
    assert!(matches!(bad_range, Err(HrError::Invalid(_))), "to_date before from_date refused");
}
