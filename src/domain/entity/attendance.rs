use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;

use super::AttendanceStatus;
use super::AuditMetadata;

/// Strongly-typed ID for Attendance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AttendanceId(pub Uuid);

impl AttendanceId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for AttendanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for AttendanceId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for AttendanceId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<AttendanceId> for Uuid {
    fn from(id: AttendanceId) -> Self { id.0 }
}

impl AsRef<Uuid> for AttendanceId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for AttendanceId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Attendance {
    pub id: Uuid,
    pub company_id: Uuid,
    pub employee_id: Uuid,
    pub attendance_date: DateTime<Utc>,
    pub status: AttendanceStatus,
    pub working_hours: Decimal,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl Attendance {
    /// Create a builder for Attendance
    pub fn builder() -> AttendanceBuilder {
        AttendanceBuilder::default()
    }

    /// Create a new Attendance with required fields
    pub fn new(company_id: Uuid, employee_id: Uuid, attendance_date: DateTime<Utc>, status: AttendanceStatus, working_hours: Decimal) -> Self {
        Self {
            id: Uuid::new_v4(),
            company_id,
            employee_id,
            attendance_date,
            status,
            working_hours,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> AttendanceId {
        AttendanceId(self.id)
    }

    /// Get when this entity was created
    pub fn created_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.created_at.as_ref()
    }

    /// Get when this entity was last updated
    pub fn updated_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.updated_at.as_ref()
    }

    /// Check if this entity is soft deleted
    pub fn is_deleted(&self) -> bool {
        self.metadata.deleted_at.is_some()
    }

    /// Check if this entity is active (not deleted)
    pub fn is_active(&self) -> bool {
        self.metadata.deleted_at.is_none()
    }

    /// Get when this entity was deleted
    pub fn deleted_at(&self) -> Option<&DateTime<Utc>> {
        self.metadata.deleted_at.as_ref()
    }

    /// Get who created this entity
    pub fn created_by(&self) -> Option<&Uuid> {
        self.metadata.created_by.as_ref()
    }

    /// Get who last updated this entity
    pub fn updated_by(&self) -> Option<&Uuid> {
        self.metadata.updated_by.as_ref()
    }

    /// Get who deleted this entity
    pub fn deleted_by(&self) -> Option<&Uuid> {
        self.metadata.deleted_by.as_ref()
    }

    /// Get the current status
    pub fn status(&self) -> &AttendanceStatus {
        &self.status
    }


    // ==========================================================
    // Partial Update
    // ==========================================================

    /// Apply partial updates from a map of field name to JSON value
    pub fn apply_patch(&mut self, fields: std::collections::HashMap<String, serde_json::Value>) {
        for (key, value) in fields {
            match key.as_str() {
                "company_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.company_id = v; }
                }
                "employee_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.employee_id = v; }
                }
                "attendance_date" => {
                    if let Ok(v) = serde_json::from_value(value) { self.attendance_date = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "working_hours" => {
                    if let Ok(v) = serde_json::from_value(value) { self.working_hours = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for Attendance {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "Attendance"
    }
}

impl backbone_core::PersistentEntity for Attendance {
    fn entity_id(&self) -> String {
        self.id.to_string()
    }
    fn set_entity_id(&mut self, id: String) {
        if let Ok(uuid) = uuid::Uuid::parse_str(&id) {
            self.id = uuid;
        }
    }
    fn created_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.created_at
    }
    fn set_created_at(&mut self, ts: chrono::DateTime<chrono::Utc>) {
        self.metadata.created_at = Some(ts);
    }
    fn updated_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.updated_at
    }
    fn set_updated_at(&mut self, ts: chrono::DateTime<chrono::Utc>) {
        self.metadata.updated_at = Some(ts);
    }
    fn deleted_at(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        self.metadata.deleted_at
    }
    fn set_deleted_at(&mut self, ts: Option<chrono::DateTime<chrono::Utc>>) {
        self.metadata.deleted_at = ts;
    }
}

impl backbone_orm::EntityRepoMeta for Attendance {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("company_id".to_string(), "uuid".to_string());
        m.insert("employee_id".to_string(), "uuid".to_string());
        m.insert("status".to_string(), "attendance_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &[]
    }
}

/// Builder for Attendance entity
///
/// Provides a fluent API for constructing Attendance instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct AttendanceBuilder {
    company_id: Option<Uuid>,
    employee_id: Option<Uuid>,
    attendance_date: Option<DateTime<Utc>>,
    status: Option<AttendanceStatus>,
    working_hours: Option<Decimal>,
}

impl AttendanceBuilder {
    /// Set the company_id field (required)
    pub fn company_id(mut self, value: Uuid) -> Self {
        self.company_id = Some(value);
        self
    }

    /// Set the employee_id field (required)
    pub fn employee_id(mut self, value: Uuid) -> Self {
        self.employee_id = Some(value);
        self
    }

    /// Set the attendance_date field (required)
    pub fn attendance_date(mut self, value: DateTime<Utc>) -> Self {
        self.attendance_date = Some(value);
        self
    }

    /// Set the status field (default: `AttendanceStatus::default()`)
    pub fn status(mut self, value: AttendanceStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the working_hours field (default: `Decimal::from(0)`)
    pub fn working_hours(mut self, value: Decimal) -> Self {
        self.working_hours = Some(value);
        self
    }

    /// Build the Attendance entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<Attendance, String> {
        let company_id = self.company_id.ok_or_else(|| "company_id is required".to_string())?;
        let employee_id = self.employee_id.ok_or_else(|| "employee_id is required".to_string())?;
        let attendance_date = self.attendance_date.ok_or_else(|| "attendance_date is required".to_string())?;

        Ok(Attendance {
            id: Uuid::new_v4(),
            company_id,
            employee_id,
            attendance_date,
            status: self.status.unwrap_or(AttendanceStatus::default()),
            working_hours: self.working_hours.unwrap_or(Decimal::from(0)),
            metadata: AuditMetadata::default(),
        })
    }
}
