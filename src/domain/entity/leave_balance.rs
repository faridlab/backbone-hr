use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;
use super::AuditMetadata;

/// Strongly-typed ID for LeaveBalance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LeaveBalanceId(pub Uuid);

impl LeaveBalanceId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for LeaveBalanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for LeaveBalanceId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for LeaveBalanceId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<LeaveBalanceId> for Uuid {
    fn from(id: LeaveBalanceId) -> Self { id.0 }
}

impl AsRef<Uuid> for LeaveBalanceId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for LeaveBalanceId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct LeaveBalance {
    pub id: Uuid,
    pub company_id: Uuid,
    pub employee_id: Uuid,
    pub leave_type_id: Uuid,
    pub year: i32,
    pub allocated: Decimal,
    pub used: Decimal,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl LeaveBalance {
    /// Create a builder for LeaveBalance
    pub fn builder() -> LeaveBalanceBuilder {
        LeaveBalanceBuilder::default()
    }

    /// Create a new LeaveBalance with required fields
    pub fn new(company_id: Uuid, employee_id: Uuid, leave_type_id: Uuid, year: i32, allocated: Decimal, used: Decimal) -> Self {
        Self {
            id: Uuid::new_v4(),
            company_id,
            employee_id,
            leave_type_id,
            year,
            allocated,
            used,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> LeaveBalanceId {
        LeaveBalanceId(self.id)
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
                "year" => {
                    if let Ok(v) = serde_json::from_value(value) { self.year = v; }
                }
                "allocated" => {
                    if let Ok(v) = serde_json::from_value(value) { self.allocated = v; }
                }
                "used" => {
                    if let Ok(v) = serde_json::from_value(value) { self.used = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for LeaveBalance {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "LeaveBalance"
    }
}

impl backbone_core::PersistentEntity for LeaveBalance {
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

impl backbone_orm::EntityRepoMeta for LeaveBalance {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("company_id".to_string(), "uuid".to_string());
        m.insert("employee_id".to_string(), "uuid".to_string());
        m.insert("leave_type_id".to_string(), "uuid".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &[]
    }
    fn company_field() -> Option<&'static str> {
        Some("company_id")
    }
}

/// Builder for LeaveBalance entity
///
/// Provides a fluent API for constructing LeaveBalance instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct LeaveBalanceBuilder {
    company_id: Option<Uuid>,
    employee_id: Option<Uuid>,
    leave_type_id: Option<Uuid>,
    year: Option<i32>,
    allocated: Option<Decimal>,
    used: Option<Decimal>,
}

impl LeaveBalanceBuilder {
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

    /// Set the year field (required)
    pub fn year(mut self, value: i32) -> Self {
        self.year = Some(value);
        self
    }

    /// Set the allocated field (default: `Decimal::from(0)`)
    pub fn allocated(mut self, value: Decimal) -> Self {
        self.allocated = Some(value);
        self
    }

    /// Set the used field (default: `Decimal::from(0)`)
    pub fn used(mut self, value: Decimal) -> Self {
        self.used = Some(value);
        self
    }

    /// Build the LeaveBalance entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<LeaveBalance, String> {
        let company_id = self.company_id.ok_or_else(|| "company_id is required".to_string())?;
        let employee_id = self.employee_id.ok_or_else(|| "employee_id is required".to_string())?;
        let leave_type_id = self.leave_type_id.ok_or_else(|| "leave_type_id is required".to_string())?;
        let year = self.year.ok_or_else(|| "year is required".to_string())?;

        Ok(LeaveBalance {
            id: Uuid::new_v4(),
            company_id,
            employee_id,
            leave_type_id,
            year,
            allocated: self.allocated.unwrap_or(Decimal::from(0)),
            used: self.used.unwrap_or(Decimal::from(0)),
            metadata: AuditMetadata::default(),
        })
    }
}
