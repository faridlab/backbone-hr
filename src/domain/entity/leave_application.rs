use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;

use super::LeaveStatus;
use super::AuditMetadata;

/// Strongly-typed ID for LeaveApplication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LeaveApplicationId(pub Uuid);

impl LeaveApplicationId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for LeaveApplicationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for LeaveApplicationId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for LeaveApplicationId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<LeaveApplicationId> for Uuid {
    fn from(id: LeaveApplicationId) -> Self { id.0 }
}

impl AsRef<Uuid> for LeaveApplicationId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for LeaveApplicationId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LeaveApplication {
    pub id: Uuid,
    pub company_id: Uuid,
    pub employee_id: Uuid,
    pub leave_type_id: Uuid,
    pub from_date: DateTime<Utc>,
    pub to_date: DateTime<Utc>,
    pub days: Decimal,
    pub status: LeaveStatus,
    pub reason: Option<String>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl LeaveApplication {
    /// Create a builder for LeaveApplication
    pub fn builder() -> LeaveApplicationBuilder {
        LeaveApplicationBuilder::default()
    }

    /// Create a new LeaveApplication with required fields
    pub fn new(company_id: Uuid, employee_id: Uuid, leave_type_id: Uuid, from_date: DateTime<Utc>, to_date: DateTime<Utc>, days: Decimal, status: LeaveStatus) -> Self {
        Self {
            id: Uuid::new_v4(),
            company_id,
            employee_id,
            leave_type_id,
            from_date,
            to_date,
            days,
            status,
            reason: None,
            approved_by: None,
            approved_at: None,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> LeaveApplicationId {
        LeaveApplicationId(self.id)
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
    pub fn status(&self) -> &LeaveStatus {
        &self.status
    }


    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the reason field (chainable)
    pub fn with_reason(mut self, value: String) -> Self {
        self.reason = Some(value);
        self
    }

    /// Set the approved_by field (chainable)
    pub fn with_approved_by(mut self, value: Uuid) -> Self {
        self.approved_by = Some(value);
        self
    }

    /// Set the approved_at field (chainable)
    pub fn with_approved_at(mut self, value: DateTime<Utc>) -> Self {
        self.approved_at = Some(value);
        self
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
                "leave_type_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.leave_type_id = v; }
                }
                "from_date" => {
                    if let Ok(v) = serde_json::from_value(value) { self.from_date = v; }
                }
                "to_date" => {
                    if let Ok(v) = serde_json::from_value(value) { self.to_date = v; }
                }
                "days" => {
                    if let Ok(v) = serde_json::from_value(value) { self.days = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "reason" => {
                    if let Ok(v) = serde_json::from_value(value) { self.reason = v; }
                }
                "approved_by" => {
                    if let Ok(v) = serde_json::from_value(value) { self.approved_by = v; }
                }
                "approved_at" => {
                    if let Ok(v) = serde_json::from_value(value) { self.approved_at = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for LeaveApplication {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "LeaveApplication"
    }
}

impl backbone_core::PersistentEntity for LeaveApplication {
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

impl backbone_orm::EntityRepoMeta for LeaveApplication {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("company_id".to_string(), "uuid".to_string());
        m.insert("employee_id".to_string(), "uuid".to_string());
        m.insert("leave_type_id".to_string(), "uuid".to_string());
        m.insert("status".to_string(), "leave_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &[]
    }
}

/// Builder for LeaveApplication entity
///
/// Provides a fluent API for constructing LeaveApplication instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct LeaveApplicationBuilder {
    company_id: Option<Uuid>,
    employee_id: Option<Uuid>,
    leave_type_id: Option<Uuid>,
    from_date: Option<DateTime<Utc>>,
    to_date: Option<DateTime<Utc>>,
    days: Option<Decimal>,
    status: Option<LeaveStatus>,
    reason: Option<String>,
    approved_by: Option<Uuid>,
    approved_at: Option<DateTime<Utc>>,
}

impl LeaveApplicationBuilder {
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

    /// Set the leave_type_id field (required)
    pub fn leave_type_id(mut self, value: Uuid) -> Self {
        self.leave_type_id = Some(value);
        self
    }

    /// Set the from_date field (required)
    pub fn from_date(mut self, value: DateTime<Utc>) -> Self {
        self.from_date = Some(value);
        self
    }

    /// Set the to_date field (required)
    pub fn to_date(mut self, value: DateTime<Utc>) -> Self {
        self.to_date = Some(value);
        self
    }

    /// Set the days field (default: `Decimal::from(0)`)
    pub fn days(mut self, value: Decimal) -> Self {
        self.days = Some(value);
        self
    }

    /// Set the status field (default: `LeaveStatus::default()`)
    pub fn status(mut self, value: LeaveStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the reason field (optional)
    pub fn reason(mut self, value: String) -> Self {
        self.reason = Some(value);
        self
    }

    /// Set the approved_by field (optional)
    pub fn approved_by(mut self, value: Uuid) -> Self {
        self.approved_by = Some(value);
        self
    }

    /// Set the approved_at field (optional)
    pub fn approved_at(mut self, value: DateTime<Utc>) -> Self {
        self.approved_at = Some(value);
        self
    }

    /// Build the LeaveApplication entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<LeaveApplication, String> {
        let company_id = self.company_id.ok_or_else(|| "company_id is required".to_string())?;
        let employee_id = self.employee_id.ok_or_else(|| "employee_id is required".to_string())?;
        let leave_type_id = self.leave_type_id.ok_or_else(|| "leave_type_id is required".to_string())?;
        let from_date = self.from_date.ok_or_else(|| "from_date is required".to_string())?;
        let to_date = self.to_date.ok_or_else(|| "to_date is required".to_string())?;

        Ok(LeaveApplication {
            id: Uuid::new_v4(),
            company_id,
            employee_id,
            leave_type_id,
            from_date,
            to_date,
            days: self.days.unwrap_or(Decimal::from(0)),
            status: self.status.unwrap_or(LeaveStatus::default()),
            reason: self.reason,
            approved_by: self.approved_by,
            approved_at: self.approved_at,
            metadata: AuditMetadata::default(),
        })
    }
}
