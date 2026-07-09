//! The hand-authored HR write path (user-owned; survives regen).
//!
//! The people master + the leave-balance engine. Posts NO GL. The load-bearing invariant is the **leave
//! balance**: approving a leave application draws down the employee's balance for that leave type/year,
//! gated so `used` never exceeds `allocated` (you cannot approve leave you don't have), and cancelling an
//! approved application restores it — the draw + the application transition commit in ONE transaction.
//! Onboarding verifies the employee's department against the REAL backbone-organization through a port
//! (zero normal Cargo edge). Days are calendar days over the inclusive range.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

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
}

impl HrWriteService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
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
        let dup: Option<Uuid> = sqlx::query_scalar(
            r#"SELECT id FROM hr.employees WHERE company_id=$1 AND employee_number=$2
               AND (metadata->>'deleted_at') IS NULL"#,
        )
        .bind(e.company_id).bind(&e.employee_number)
        .fetch_optional(&self.pool)
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
        sqlx::query(
            r#"INSERT INTO hr.employees
                 (id, company_id, employee_number, user_id, department_id, first_name, last_name,
                  designation, employment_type, date_of_joining, status, nik, npwp, tax_status,
                  bank_account_no, base_salary)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9::employment_type,$10,'active'::employee_status,
                       $11,$12,$13::tax_status,$14,$15)"#,
        )
        .bind(id).bind(e.company_id).bind(&e.employee_number).bind(e.user_id).bind(e.department_id)
        .bind(&e.first_name).bind(&e.last_name).bind(&e.designation).bind(&e.employment_type)
        .bind(e.date_of_joining).bind(&e.nik).bind(&e.npwp).bind(&e.tax_status).bind(&e.bank_account_no)
        .bind(e.base_salary)
        .execute(&self.pool)
        .await?;
        sink.publish(&HrEvent::EmployeeOnboarded(EmployeeOnboarded {
            employee_id: id, company_id: e.company_id, employee_number: e.employee_number,
        }));
        Ok(id)
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
        let row = sqlx::query(
            r#"UPDATE hr.employees SET status=$2::employee_status, date_of_exit=$3
               WHERE id=$1 AND status='active'::employee_status AND (metadata->>'deleted_at') IS NULL
               RETURNING company_id"#,
        )
        .bind(employee_id).bind(status).bind(exit_date)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(r) => {
                sink.publish(&HrEvent::EmployeeExited(EmployeeExited {
                    employee_id, company_id: r.get("company_id"), terminated,
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
        sqlx::query(
            r#"INSERT INTO hr.leave_types (id, company_id, name, is_paid, annual_quota_days, allow_carry_forward, is_active)
               VALUES ($1,$2,$3,$4,$5,$6,true)"#,
        )
        .bind(id).bind(t.company_id).bind(&t.name).bind(t.is_paid).bind(t.annual_quota_days).bind(t.allow_carry_forward)
        .execute(&self.pool)
        .await?;
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
        let moved = sqlx::query(
            r#"INSERT INTO hr.leave_balances (id, company_id, employee_id, leave_type_id, year, allocated, used)
               VALUES ($1,$2,$3,$4,$5,$6,0)
               ON CONFLICT (employee_id, leave_type_id, year)
               DO UPDATE SET allocated = $6
               WHERE hr.leave_balances.used <= $6"#,
        )
        .bind(Uuid::new_v4()).bind(company_id).bind(employee_id).bind(leave_type_id).bind(year).bind(days)
        .execute(&self.pool)
        .await?;
        if moved.rows_affected() != 1 {
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
        let emp = sqlx::query(
            r#"SELECT company_id, status::text AS status FROM hr.employees
               WHERE id=$1 AND (metadata->>'deleted_at') IS NULL"#,
        )
        .bind(a.employee_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(HrError::NotFound("employee"))?;
        if emp.get::<String, _>("status") != "active" {
            return Err(HrError::InvalidState("employee is not active"));
        }
        let company_id: Uuid = emp.get("company_id");
        let active: Option<bool> = sqlx::query_scalar(
            "SELECT is_active FROM hr.leave_types WHERE id=$1 AND company_id=$2")
            .bind(a.leave_type_id).bind(company_id)
            .fetch_optional(&self.pool)
            .await?;
        match active {
            None => return Err(HrError::NotFound("leave type")),
            Some(false) => return Err(HrError::InvalidState("leave type is not active")),
            Some(true) => {}
        }
        let days = Decimal::from((a.to_date.date_naive() - a.from_date.date_naive()).num_days() + 1);
        let id = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO hr.leave_applications
                 (id, company_id, employee_id, leave_type_id, from_date, to_date, days, status, reason)
               VALUES ($1,$2,$3,$4,$5,$6,$7,'pending'::leave_status,$8)"#,
        )
        .bind(id).bind(company_id).bind(a.employee_id).bind(a.leave_type_id).bind(a.from_date)
        .bind(a.to_date).bind(days).bind(&a.reason)
        .execute(&self.pool)
        .await?;
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
        let app = sqlx::query(
            r#"SELECT la.company_id, la.employee_id, la.leave_type_id, la.days, la.from_date,
                      la.status::text AS status, lt.is_paid
               FROM hr.leave_applications la JOIN hr.leave_types lt ON lt.id = la.leave_type_id
               WHERE la.id=$1 AND (la.metadata->>'deleted_at') IS NULL"#,
        )
        .bind(leave_application_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(HrError::NotFound("leave application"))?;
        if app.get::<String, _>("status") != "pending" {
            return Err(HrError::InvalidState("leave application is not pending"));
        }
        let company_id: Uuid = app.get("company_id");
        let employee_id: Uuid = app.get("employee_id");
        let leave_type_id: Uuid = app.get("leave_type_id");
        let days: Decimal = app.get("days");
        let is_paid: bool = app.get("is_paid");
        let year = app.get::<DateTime<Utc>, _>("from_date").date_naive().format("%Y").to_string().parse::<i32>().unwrap();

        let mut tx = self.pool.begin().await?;
        // Claim the transition first (write-once), then draw the balance under the same tx.
        let moved = sqlx::query(
            r#"UPDATE hr.leave_applications
               SET status='approved'::leave_status, approved_by=$2, approved_at=$3
               WHERE id=$1 AND status='pending'::leave_status"#,
        )
        .bind(leave_application_id).bind(approver).bind(now)
        .execute(&mut *tx)
        .await?;
        if moved.rows_affected() != 1 {
            tx.rollback().await?;
            return Err(HrError::InvalidState("leave application is not pending"));
        }
        // Gate on availability: draw only if used + days <= allocated.
        let drawn = sqlx::query(
            r#"UPDATE hr.leave_balances SET used = used + $4
               WHERE employee_id=$1 AND leave_type_id=$2 AND year=$3 AND used + $4 <= allocated"#,
        )
        .bind(employee_id).bind(leave_type_id).bind(year).bind(days)
        .execute(&mut *tx)
        .await?;
        if drawn.rows_affected() != 1 {
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
        let moved = sqlx::query(
            r#"UPDATE hr.leave_applications SET status='rejected'::leave_status
               WHERE id=$1 AND status='pending'::leave_status AND (metadata->>'deleted_at') IS NULL"#,
        )
        .bind(leave_application_id)
        .execute(&self.pool)
        .await?;
        if moved.rows_affected() != 1 {
            return Err(HrError::InvalidState("leave application is not pending"));
        }
        Ok(())
    }

    /// Cancel a leave application. If it was APPROVED, restores the drawn-down balance in the same tx as
    /// the transition (so a balance is never left short); a pending one just cancels.
    pub async fn cancel_leave(&self, leave_application_id: Uuid) -> Result<(), HrError> {
        let app = sqlx::query(
            r#"SELECT employee_id, leave_type_id, days, from_date, status::text AS status
               FROM hr.leave_applications WHERE id=$1 AND (metadata->>'deleted_at') IS NULL"#,
        )
        .bind(leave_application_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(HrError::NotFound("leave application"))?;
        let status: String = app.get("status");
        if status == "pending" {
            let m = sqlx::query(
                "UPDATE hr.leave_applications SET status='cancelled'::leave_status WHERE id=$1 AND status='pending'::leave_status")
                .bind(leave_application_id).execute(&self.pool).await?;
            return if m.rows_affected() == 1 { Ok(()) } else { Err(HrError::InvalidState("not cancellable")) };
        }
        if status != "approved" {
            return Err(HrError::InvalidState("only a pending or approved application can be cancelled"));
        }
        let employee_id: Uuid = app.get("employee_id");
        let leave_type_id: Uuid = app.get("leave_type_id");
        let days: Decimal = app.get("days");
        let year = app.get::<DateTime<Utc>, _>("from_date").date_naive().format("%Y").to_string().parse::<i32>().unwrap();

        let mut tx = self.pool.begin().await?;
        let moved = sqlx::query(
            r#"UPDATE hr.leave_applications SET status='cancelled'::leave_status
               WHERE id=$1 AND status='approved'::leave_status"#,
        )
        .bind(leave_application_id)
        .execute(&mut *tx)
        .await?;
        if moved.rows_affected() != 1 {
            tx.rollback().await?;
            return Err(HrError::InvalidState("leave application is not approved"));
        }
        // Restore is GATED on `used >= days` so a tampered application (its `days` mutated after approval
        // via the generic PATCH surface) can never drive `used` negative and manufacture phantom
        // entitlement (maturity council 2026-07-08). The DB CHECK `used >= 0` is the backstop for any
        // other writer; this gate turns the violation into a clean domain error instead of a raw error.
        let restored = sqlx::query(
            r#"UPDATE hr.leave_balances SET used = used - $4
               WHERE employee_id=$1 AND leave_type_id=$2 AND year=$3 AND used >= $4"#,
        )
        .bind(employee_id).bind(leave_type_id).bind(year).bind(days)
        .execute(&mut *tx)
        .await?;
        if restored.rows_affected() != 1 {
            tx.rollback().await?;
            return Err(HrError::InvalidState("cannot restore leave balance — the application's days exceed what was drawn"));
        }
        tx.commit().await?;
        Ok(())
    }

    /// Record (or overwrite) an employee's attendance for a day. One record per (employee, date).
    pub async fn mark_attendance(&self, a: NewAttendance) -> Result<Uuid, HrError> {
        let emp = sqlx::query(
            "SELECT company_id FROM hr.employees WHERE id=$1 AND (metadata->>'deleted_at') IS NULL")
            .bind(a.employee_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(HrError::NotFound("employee"))?;
        let company_id: Uuid = emp.get("company_id");
        let id: Uuid = sqlx::query_scalar(
            r#"INSERT INTO hr.attendances (id, company_id, employee_id, attendance_date, status, working_hours)
               VALUES ($1,$2,$3,$4,$5::attendance_status,$6)
               ON CONFLICT (employee_id, attendance_date)
               DO UPDATE SET status=EXCLUDED.status, working_hours=EXCLUDED.working_hours
               RETURNING id"#,
        )
        .bind(Uuid::new_v4()).bind(company_id).bind(a.employee_id).bind(a.attendance_date)
        .bind(&a.status).bind(a.working_hours)
        .fetch_one(&self.pool)
        .await?;
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
        // Approved leave days clamped to the period, split by is_paid.
        let rows = sqlx::query(
            r#"SELECT lt.is_paid,
                      COALESCE(SUM(GREATEST(0, (LEAST(la.to_date::date, $3::date)
                                              - GREATEST(la.from_date::date, $2::date) + 1))), 0)::numeric AS days
               FROM hr.leave_applications la JOIN hr.leave_types lt ON lt.id = la.leave_type_id
               WHERE la.employee_id=$1 AND la.status='approved'::leave_status
                 AND (la.metadata->>'deleted_at') IS NULL
                 AND la.from_date::date <= $3::date AND la.to_date::date >= $2::date
               GROUP BY lt.is_paid"#,
        )
        .bind(employee_id).bind(from).bind(to)
        .fetch_all(&self.pool)
        .await?;
        let (mut paid, mut unpaid) = (Decimal::ZERO, Decimal::ZERO);
        for r in &rows {
            let d: Decimal = r.get("days");
            if r.get::<bool, _>("is_paid") { paid += d; } else { unpaid += d; }
        }
        // Attendance absences NOT already covered by an approved leave (so leave + attendance never
        // double-count the same day).
        let absent_days: i64 = sqlx::query_scalar(
            r#"SELECT count(*) FROM hr.attendances a
               WHERE a.employee_id=$1 AND a.attendance_date::date BETWEEN $2::date AND $3::date
                 AND a.status='absent'::attendance_status AND (a.metadata->>'deleted_at') IS NULL
                 AND NOT EXISTS (
                   SELECT 1 FROM hr.leave_applications la
                   WHERE la.employee_id=$1 AND la.status='approved'::leave_status
                     AND a.attendance_date::date BETWEEN la.from_date::date AND la.to_date::date)"#,
        )
        .bind(employee_id).bind(from).bind(to)
        .fetch_one(&self.pool)
        .await?;
        Ok(PeriodSummary { paid_leave_days: paid, unpaid_leave_days: unpaid, absent_days })
    }
}
