//! The hand-authored HR write path (user-owned; survives regen).
//!
//! The people master + the leave-balance engine. Posts NO GL. The load-bearing invariant is the **leave
//! balance**: approving a leave application draws down the employee's balance for that leave type/year,
//! gated so `used` never exceeds `allocated` (you cannot approve leave you don't have), and cancelling an
//! approved application restores it — the draw + the application transition commit in ONE transaction.
//! Onboarding verifies the employee's department against the REAL backbone-organization through a port
//! (zero normal Cargo edge). Days are calendar days over the inclusive range.

use backbone_orm::company_scope;
use chrono::{DateTime, Utc};
use chrono::Datelike;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::infrastructure::persistence::{
    AttendanceRepository, EmployeeRepository, LeaveApplicationRepository, LeaveBalanceRepository,
    LeaveTypeRepository, NewAllocationRow, NewAttendanceRow, NewEmployeeRow, NewLeaveApplicationRow,
    NewLeaveTypeRow,
};

use super::hr_events::*;
use super::hr_ports::*;

#[derive(Debug, thiserror::Error)]
pub enum HrError {
    #[error("db: {0}")]
    Db(#[from] sqlx::Error),
    #[error("not found: {0}")]
    NotFound(&'static str),
    #[error("invalid state: {0}")]
    InvalidState(&'static str),
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("insufficient leave balance")]
    InsufficientBalance,
    #[error("organization rejected: {0}")]
    OrgRejected(String),
}

pub struct NewEmployee {
    pub company_id: Uuid,
    pub employee_number: String,
    pub user_id: Option<Uuid>,
    pub department_id: Option<Uuid>,
    pub first_name: String,
    pub last_name: Option<String>,
    pub designation: Option<String>,
    pub employment_type: String, // employment_type variant
    pub date_of_joining: DateTime<Utc>,
    pub nik: Option<String>,
    pub npwp: Option<String>,
    pub tax_status: String, // tax_status variant
    pub bank_account_no: Option<String>,
    pub base_salary: Decimal,
}

pub struct NewLeaveType {
    pub company_id: Uuid,
    pub name: String,
    pub is_paid: bool,
    pub annual_quota_days: Decimal,
    pub allow_carry_forward: bool,
}

pub struct NewLeaveApplication {
    pub employee_id: Uuid,
    pub leave_type_id: Uuid,
    pub from_date: DateTime<Utc>,
    pub to_date: DateTime<Utc>,
    pub reason: Option<String>,
}

pub struct NewAttendance {
    pub employee_id: Uuid,
    pub attendance_date: DateTime<Utc>,
    pub status: String, // attendance_status variant
    pub working_hours: Decimal,
}

/// The payroll-facing OUTPUT for one employee over a pay period — the single reconciled source of the
/// days that scale pay. `unpaid_leave_days + absent_days` is what payroll deducts; paid leave does not
/// cut pay. Leave days are authoritative (from approved applications, clamped to the period); `absent_
/// days` counts only attendance absences NOT already covered by an approved leave, so the two channels
/// never double-count (completeness council 2026-07-08).
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodSummary {
    pub paid_leave_days: Decimal,
    pub unpaid_leave_days: Decimal,
    pub absent_days: i64,
}

pub struct HrWriteService {
    pool: PgPool,
    employees: EmployeeRepository,
    leave_types: LeaveTypeRepository,
    leave_balances: LeaveBalanceRepository,
    leave_applications: LeaveApplicationRepository,
    attendances: AttendanceRepository,
}

impl HrWriteService {
    pub fn new(pool: PgPool) -> Self {
        let employees = EmployeeRepository::new(pool.clone());
        let leave_types = LeaveTypeRepository::new(pool.clone());
        let leave_balances = LeaveBalanceRepository::new(pool.clone());
        let leave_applications = LeaveApplicationRepository::new(pool.clone());
        let attendances = AttendanceRepository::new(pool.clone());
        Self { pool, employees, leave_types, leave_balances, leave_applications, attendances }
    }

    /// Onboard an employee. `employee_number` must be unique per company; if a `department_id` is given
    /// it is verified against the REAL organization (must exist AND belong to the employee's company).
    /// Emits `EmployeeOnboarded`.
    pub async fn onboard_employee(
        &self,
        e: NewEmployee,
        org: &dyn OrgPort,
        sink: &dyn HrEventSink,
    ) -> Result<Uuid, HrError> {
        if e.employee_number.trim().is_empty() {
            return Err(HrError::Invalid("employee needs a number".into()));
        }
        if e.first_name.trim().is_empty() {
            return Err(HrError::Invalid("employee needs a name".into()));
        }
        // RLS scope (ADR-0008): the company is on the DTO — bind it for the whole body so the
        // uniqueness probe and the insert both run with `app.company_id` set. The explicit
        // `company_id` filter/bind stay as defense-in-depth.
        let company = e.company_id;
        company_scope::with_company_scope(Some(company), async move {
        let dup = self.employees
            .find_live_id_by_number(&self.pool, e.company_id, &e.employee_number)
            .await?;
        if dup.is_some() {
            return Err(HrError::Invalid("employee_number already exists in this company".into()));
        }
        if let Some(dept) = e.department_id {
            let d = org.resolve_department(dept).await.map_err(|r| HrError::OrgRejected(r.code))?;
            if d.company_id != e.company_id {
                return Err(HrError::Invalid("department belongs to a different company".into()));
            }
        }
        let id = Uuid::new_v4();
        self.employees.insert_employee(&self.pool, &NewEmployeeRow {
            id,
            company_id: e.company_id,
            employee_number: &e.employee_number,
            user_id: e.user_id,
            department_id: e.department_id,
            first_name: &e.first_name,
            last_name: e.last_name.as_deref(),
            designation: e.designation.as_deref(),
            employment_type: &e.employment_type,
            date_of_joining: e.date_of_joining,
            nik: e.nik.as_deref(),
            npwp: e.npwp.as_deref(),
            tax_status: &e.tax_status,
            bank_account_no: e.bank_account_no.as_deref(),
            base_salary: e.base_salary,
        }).await?;
        sink.publish(&HrEvent::EmployeeOnboarded(EmployeeOnboarded {
            employee_id: id, company_id: e.company_id, employee_number: e.employee_number,
        }));
        Ok(id)
        }).await
    }

    /// Exit an employee (resign or terminate) — sets the exit date and stops the roster. Terminal.
    pub async fn exit_employee(
        &self,
        employee_id: Uuid,
        terminated: bool,
        exit_date: DateTime<Utc>,
        sink: &dyn HrEventSink,
    ) -> Result<(), HrError> {
        let status = if terminated { "terminated" } else { "resigned" };
        // RLS scope (ADR-0008), ID-only pattern: identified by the employee id alone — there is no
        // company argument. The write rides the REQUEST-dedicated connection (which carries the
        // caller's `app.company_id`), so another company's employee simply isn't matched.
        let company_id = self.employees.exit_employee(&self.pool, employee_id, status, exit_date).await?;
        match company_id {
            Some(company_id) => {
                sink.publish(&HrEvent::EmployeeExited(EmployeeExited {
                    employee_id, company_id, terminated,
                }));
                Ok(())
            }
            None => Err(HrError::InvalidState("employee is not active")),
        }
    }

    /// Define a leave type.
    pub async fn create_leave_type(&self, t: NewLeaveType) -> Result<Uuid, HrError> {
        if t.name.trim().is_empty() {
            return Err(HrError::Invalid("leave type needs a name".into()));
        }
        if t.annual_quota_days < Decimal::ZERO {
            return Err(HrError::Invalid("quota must be non-negative".into()));
        }
        let id = Uuid::new_v4();
        // RLS scope (ADR-0008): company on the DTO — bind it so the insert passes the WITH CHECK.
        company_scope::with_company_scope(Some(t.company_id), self.leave_types.insert_leave_type(
            &self.pool,
            &NewLeaveTypeRow {
                id,
                company_id: t.company_id,
                name: &t.name,
                is_paid: t.is_paid,
                annual_quota_days: t.annual_quota_days,
                allow_carry_forward: t.allow_carry_forward,
            },
        )).await?;
        Ok(id)
    }

    /// Allocate (set) an employee's leave entitlement for a type in a year. Idempotent per
    /// (employee, type, year): a re-allocation overwrites `allocated` (never below what's already used).
    pub async fn allocate_leave(
        &self,
        company_id: Uuid,
        employee_id: Uuid,
        leave_type_id: Uuid,
        year: i32,
        days: Decimal,
    ) -> Result<(), HrError> {
        if days < Decimal::ZERO {
            return Err(HrError::Invalid("allocation must be non-negative".into()));
        }
        // RLS scope (ADR-0008): company is an explicit parameter — bind it for the upsert.
        let moved = company_scope::with_company_scope(Some(company_id), self.leave_balances.upsert_allocation(
            &self.pool,
            &NewAllocationRow {
                id: Uuid::new_v4(),
                company_id,
                employee_id,
                leave_type_id,
                year,
                allocated: days,
            },
        )).await?;
        if moved != 1 {
            return Err(HrError::Invalid("allocation is below the days already used".into()));
        }
        Ok(())
    }

    /// Apply for leave. Computes `days` as the inclusive calendar-day span; the request is `pending`
    /// until approved. Requires an ACTIVE employee, an active leave type, and `from <= to`.
    pub async fn apply_leave(&self, a: NewLeaveApplication) -> Result<Uuid, HrError> {
        if a.to_date < a.from_date {
            return Err(HrError::Invalid("to_date is before from_date".into()));
        }
        // Year-spanning leave is rejected: the leave engine debits exactly one year's allocation
        // (the `from_date` year, see approve_leave), so a Dec→Jan application would silently debit
        // only the start year's bucket and leave the Jan days "free" (ADR-001 parking lot). Callers
        // must split the application at the calendar-year boundary — one application per year.
        if a.from_date.date_naive().year() != a.to_date.date_naive().year() {
            return Err(HrError::Invalid(
                "leave application spans calendar years; file one application per year".into(),
            ));
        }
        // RLS scope (ADR-0008), ID-only pattern: the applicant is identified by employee id alone, so
        // the lookup rides the request-dedicated connection (RLS fences it to the caller's company).
        // Having read the employee's company off the row, the insert below is bound to it explicitly.
        let emp = self.employees.find_scope_by_id(&self.pool, a.employee_id).await?
            .ok_or(HrError::NotFound("employee"))?;
        if emp.status != "active" {
            return Err(HrError::InvalidState("employee is not active"));
        }
        let company_id = emp.company_id;
        let active = company_scope::with_company_scope(
            Some(company_id),
            self.leave_types.find_is_active(&self.pool, a.leave_type_id, company_id),
        ).await?;
        match active {
            None => return Err(HrError::NotFound("leave type")),
            Some(false) => return Err(HrError::InvalidState("leave type is not active")),
            Some(true) => {}
        }
        let days = Decimal::from((a.to_date.date_naive() - a.from_date.date_naive()).num_days() + 1);
        let id = Uuid::new_v4();
        company_scope::with_company_scope(Some(company_id), self.leave_applications.insert_application(
            &self.pool,
            &NewLeaveApplicationRow {
                id,
                company_id,
                employee_id: a.employee_id,
                leave_type_id: a.leave_type_id,
                from_date: a.from_date,
                to_date: a.to_date,
                days,
                reason: a.reason.as_deref(),
            },
        )).await?;
        Ok(id)
    }

    /// Approve a leave application — THE invariant. Draws down the employee's balance for the leave
    /// type/year, GATED so `used` never exceeds `allocated`, in the SAME transaction as the
    /// `pending → approved` transition. If the balance is insufficient (or missing), both are rolled back
    /// and nothing changes. Emits `LeaveApproved`.
    pub async fn approve_leave(
        &self,
        leave_application_id: Uuid,
        approver: Option<Uuid>,
        now: DateTime<Utc>,
        sink: &dyn HrEventSink,
    ) -> Result<(), HrError> {
        // RLS scope (ADR-0008), ID-only pattern: identified by the application id alone. The read rides
        // the request-dedicated connection; the company read off the row then binds the transaction
        // below, so the transition + the balance draw are both fenced even for non-request callers.
        let app = self.leave_applications.find_for_approval(&self.pool, leave_application_id).await?
            .ok_or(HrError::NotFound("leave application"))?;
        if app.status != "pending" {
            return Err(HrError::InvalidState("leave application is not pending"));
        }
        let company_id = app.company_id;
        let employee_id = app.employee_id;
        let leave_type_id = app.leave_type_id;
        let days = app.days;
        let is_paid = app.is_paid;
        let year = app.from_date.date_naive().year();

        let mut tx = self.pool.begin().await?;
        company_scope::bind_company_on(&mut tx, company_id).await?;
        // Claim the transition first (write-once), then draw the balance under the same tx.
        let moved = self.leave_applications
            .mark_approved(&mut tx, leave_application_id, approver, now)
            .await?;
        if moved != 1 {
            tx.rollback().await?;
            return Err(HrError::InvalidState("leave application is not pending"));
        }
        // Gate on availability: draw only if used + days <= allocated.
        let drawn = self.leave_balances
            .draw(&mut tx, employee_id, leave_type_id, year, days)
            .await?;
        if drawn != 1 {
            tx.rollback().await?;
            return Err(HrError::InsufficientBalance);
        }
        tx.commit().await?;
        sink.publish(&HrEvent::LeaveApproved(LeaveApproved {
            leave_application_id, employee_id, company_id, leave_type_id, days, is_paid,
        }));
        Ok(())
    }

    /// Reject a pending leave application (no balance change).
    pub async fn reject_leave(&self, leave_application_id: Uuid) -> Result<(), HrError> {
        // RLS scope (ADR-0008), ID-only pattern: no company argument — the write rides the
        // request-dedicated connection, so another company's application is simply not matched.
        let moved = self.leave_applications.mark_rejected(&self.pool, leave_application_id).await?;
        if moved != 1 {
            return Err(HrError::InvalidState("leave application is not pending"));
        }
        Ok(())
    }

    /// Cancel a leave application. If it was APPROVED, restores the drawn-down balance in the same tx as
    /// the transition (so a balance is never left short); a pending one just cancels.
    pub async fn cancel_leave(&self, leave_application_id: Uuid) -> Result<(), HrError> {
        // RLS scope (ADR-0008), ID-only pattern: identified by the application id alone. The read rides
        // the request-dedicated connection and now also carries `company_id`, so the restore
        // transaction below can be bound explicitly (correct for non-request callers too).
        let app = self.leave_applications.find_for_cancel(&self.pool, leave_application_id).await?
            .ok_or(HrError::NotFound("leave application"))?;
        let company_id = app.company_id;
        let status = app.status.as_str();
        if status == "pending" {
            let m = self.leave_applications.cancel_pending(&self.pool, leave_application_id).await?;
            return if m == 1 { Ok(()) } else { Err(HrError::InvalidState("not cancellable")) };
        }
        if status != "approved" {
            return Err(HrError::InvalidState("only a pending or approved application can be cancelled"));
        }
        let employee_id = app.employee_id;
        let leave_type_id = app.leave_type_id;
        let days = app.days;
        let year = app.from_date.date_naive().year();

        let mut tx = self.pool.begin().await?;
        company_scope::bind_company_on(&mut tx, company_id).await?;
        let moved = self.leave_applications.cancel_approved(&mut tx, leave_application_id).await?;
        if moved != 1 {
            tx.rollback().await?;
            return Err(HrError::InvalidState("leave application is not approved"));
        }
        // Restore is GATED on `used >= days` so a tampered application (its `days` mutated after approval
        // via the generic PATCH surface) can never drive `used` negative and manufacture phantom
        // entitlement (maturity council 2026-07-08). The DB CHECK `used >= 0` is the backstop for any
        // other writer; this gate turns the violation into a clean domain error instead of a raw error.
        let restored = self.leave_balances
            .restore(&mut tx, employee_id, leave_type_id, year, days)
            .await?;
        if restored != 1 {
            tx.rollback().await?;
            return Err(HrError::InvalidState("cannot restore leave balance — the application's days exceed what was drawn"));
        }
        tx.commit().await?;
        Ok(())
    }

    /// Record (or overwrite) an employee's attendance for a day. One record per (employee, date).
    pub async fn mark_attendance(&self, a: NewAttendance) -> Result<Uuid, HrError> {
        // RLS scope (ADR-0008), ID-only pattern: the employee lookup rides the request-dedicated
        // connection; the company read off that row then scopes the upsert (so it passes WITH CHECK).
        let company_id = self.employees.find_company_by_id(&self.pool, a.employee_id).await?
            .ok_or(HrError::NotFound("employee"))?;
        let id = company_scope::with_company_scope(Some(company_id), self.attendances.upsert_attendance(
            &self.pool,
            &NewAttendanceRow {
                id: Uuid::new_v4(),
                company_id,
                employee_id: a.employee_id,
                attendance_date: a.attendance_date,
                status: &a.status,
                working_hours: a.working_hours,
            },
        )).await?;
        Ok(id)
    }

    /// The payroll-facing read: reconcile approved leave + attendance into the days that scale an
    /// employee's pay over `[from, to]` (inclusive dates). This is the input `backbone-payroll` consumes —
    /// without it, payroll would have to guess paid-vs-unpaid from the event and double-count leave
    /// against attendance (completeness council 2026-07-08).
    pub async fn period_summary(
        &self,
        employee_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<PeriodSummary, HrError> {
        // RLS scope (ADR-0008), ID-only pattern: read-only, identified by employee id alone — both
        // reads ride the request-dedicated connection. An event/job caller must wrap this in
        // `with_company_scope(Some(company_id))`, otherwise the reads fail closed (0 rows).
        // Approved leave days clamped to the period, split by is_paid.
        let rows = self.leave_applications
            .sum_approved_days_by_paid(&self.pool, employee_id, from, to)
            .await?;
        let (mut paid, mut unpaid) = (Decimal::ZERO, Decimal::ZERO);
        for r in &rows {
            if r.is_paid { paid += r.days; } else { unpaid += r.days; }
        }
        // Attendance absences NOT already covered by an approved leave (so leave + attendance never
        // double-count the same day).
        let absent_days = self.attendances
            .count_uncovered_absences(&self.pool, employee_id, from, to)
            .await?;
        Ok(PeriodSummary { paid_leave_days: paid, unpaid_leave_days: unpaid, absent_days })
    }
}
