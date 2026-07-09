//! HR domain events (hand-authored, user-owned) — the public extension surface.
//!
//! backbone-hr posts NO GL. Its events are read-side signals for downstream consumers: an employee
//! joined/left the workforce (payroll's roster changes), and a leave was approved (payroll's paid/unpaid
//! day computation). A consuming service supplies the sink (bus, outbox, …).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An employee was onboarded — they now exist in the roster (payroll can include them).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmployeeOnboarded {
    pub employee_id: Uuid,
    pub company_id: Uuid,
    pub employee_number: String,
}

/// An employee left the workforce — payroll should stop including them after the exit date.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmployeeExited {
    pub employee_id: Uuid,
    pub company_id: Uuid,
    pub terminated: bool,
}

/// A leave application was approved and its balance drawn down. `is_paid` is the decisive payroll signal:
/// unpaid leave (`is_paid=false`) cuts pay, paid leave does not — so payroll never has to guess.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LeaveApproved {
    pub leave_application_id: Uuid,
    pub employee_id: Uuid,
    pub company_id: Uuid,
    pub leave_type_id: Uuid,
    pub days: Decimal,
    pub is_paid: bool,
}

/// The HR domain-event union.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum HrEvent {
    EmployeeOnboarded(EmployeeOnboarded),
    EmployeeExited(EmployeeExited),
    LeaveApproved(LeaveApproved),
}

/// Sink the write path publishes to. A consuming service supplies its own (bus, outbox, …).
pub trait HrEventSink: Send + Sync {
    fn publish(&self, event: &HrEvent);
}

/// A no-op/logging sink for tests and single-process composition.
#[derive(Debug, Default, Clone)]
pub struct LoggingSink;

impl HrEventSink for LoggingSink {
    fn publish(&self, event: &HrEvent) {
        tracing::info!(?event, "hr event");
    }
}
