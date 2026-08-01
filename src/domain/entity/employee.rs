use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use rust_decimal::Decimal;

use super::EmploymentType;
use super::EmployeeStatus;
use super::TaxStatus;
use super::AuditMetadata;

/// Strongly-typed ID for Employee
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EmployeeId(pub Uuid);

impl EmployeeId {
    pub fn new(id: Uuid) -> Self { Self(id) }
    pub fn generate() -> Self { Self(Uuid::new_v4()) }
    pub fn into_inner(self) -> Uuid { self.0 }
}

impl std::fmt::Display for EmployeeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for EmployeeId {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<Uuid> for EmployeeId {
    fn from(id: Uuid) -> Self { Self(id) }
}

impl From<EmployeeId> for Uuid {
    fn from(id: EmployeeId) -> Self { id.0 }
}

impl AsRef<Uuid> for EmployeeId {
    fn as_ref(&self) -> &Uuid { &self.0 }
}

impl std::ops::Deref for EmployeeId {
    type Target = Uuid;
    fn deref(&self) -> &Self::Target { &self.0 }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Employee {
    pub id: Uuid,
    pub company_id: Uuid,
    pub employee_number: String,
    pub user_id: Option<Uuid>,
    pub department_id: Option<Uuid>,
    pub first_name: String,
    pub last_name: Option<String>,
    pub designation: Option<String>,
    pub employment_type: EmploymentType,
    pub date_of_joining: DateTime<Utc>,
    pub date_of_exit: Option<DateTime<Utc>>,
    pub status: EmployeeStatus,
    pub nik: Option<String>,
    pub npwp: Option<String>,
    pub tax_status: TaxStatus,
    pub bank_account_no: Option<String>,
    pub base_salary: Decimal,
    #[serde(default)]
    #[sqlx(json)]
    pub metadata: AuditMetadata,
}

impl Employee {
    /// Create a builder for Employee
    pub fn builder() -> EmployeeBuilder {
        EmployeeBuilder::default()
    }

    /// Create a new Employee with required fields
    pub fn new(company_id: Uuid, employee_number: String, first_name: String, employment_type: EmploymentType, date_of_joining: DateTime<Utc>, status: EmployeeStatus, tax_status: TaxStatus, base_salary: Decimal) -> Self {
        Self {
            id: Uuid::new_v4(),
            company_id,
            employee_number,
            user_id: None,
            department_id: None,
            first_name,
            last_name: None,
            designation: None,
            employment_type,
            date_of_joining,
            date_of_exit: None,
            status,
            nik: None,
            npwp: None,
            tax_status,
            bank_account_no: None,
            base_salary,
            metadata: AuditMetadata::default(),
        }
    }

    /// Get the entity's unique identifier
    pub fn id(&self) -> &Uuid {
        &self.id
    }

    /// Get a strongly-typed ID for this entity
    pub fn typed_id(&self) -> EmployeeId {
        EmployeeId(self.id)
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
    pub fn status(&self) -> &EmployeeStatus {
        &self.status
    }


    // ==========================================================
    // Fluent Setters (with_* for optional fields)
    // ==========================================================

    /// Set the user_id field (chainable)
    pub fn with_user_id(mut self, value: Uuid) -> Self {
        self.user_id = Some(value);
        self
    }

    /// Set the department_id field (chainable)
    pub fn with_department_id(mut self, value: Uuid) -> Self {
        self.department_id = Some(value);
        self
    }

    /// Set the last_name field (chainable)
    pub fn with_last_name(mut self, value: String) -> Self {
        self.last_name = Some(value);
        self
    }

    /// Set the designation field (chainable)
    pub fn with_designation(mut self, value: String) -> Self {
        self.designation = Some(value);
        self
    }

    /// Set the date_of_exit field (chainable)
    pub fn with_date_of_exit(mut self, value: DateTime<Utc>) -> Self {
        self.date_of_exit = Some(value);
        self
    }

    /// Set the nik field (chainable)
    pub fn with_nik(mut self, value: String) -> Self {
        self.nik = Some(value);
        self
    }

    /// Set the npwp field (chainable)
    pub fn with_npwp(mut self, value: String) -> Self {
        self.npwp = Some(value);
        self
    }

    /// Set the bank_account_no field (chainable)
    pub fn with_bank_account_no(mut self, value: String) -> Self {
        self.bank_account_no = Some(value);
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
                "employee_number" => {
                    if let Ok(v) = serde_json::from_value(value) { self.employee_number = v; }
                }
                "user_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.user_id = v; }
                }
                "department_id" => {
                    if let Ok(v) = serde_json::from_value(value) { self.department_id = v; }
                }
                "first_name" => {
                    if let Ok(v) = serde_json::from_value(value) { self.first_name = v; }
                }
                "last_name" => {
                    if let Ok(v) = serde_json::from_value(value) { self.last_name = v; }
                }
                "designation" => {
                    if let Ok(v) = serde_json::from_value(value) { self.designation = v; }
                }
                "employment_type" => {
                    if let Ok(v) = serde_json::from_value(value) { self.employment_type = v; }
                }
                "date_of_joining" => {
                    if let Ok(v) = serde_json::from_value(value) { self.date_of_joining = v; }
                }
                "date_of_exit" => {
                    if let Ok(v) = serde_json::from_value(value) { self.date_of_exit = v; }
                }
                "status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.status = v; }
                }
                "nik" => {
                    if let Ok(v) = serde_json::from_value(value) { self.nik = v; }
                }
                "npwp" => {
                    if let Ok(v) = serde_json::from_value(value) { self.npwp = v; }
                }
                "tax_status" => {
                    if let Ok(v) = serde_json::from_value(value) { self.tax_status = v; }
                }
                "bank_account_no" => {
                    if let Ok(v) = serde_json::from_value(value) { self.bank_account_no = v; }
                }
                "base_salary" => {
                    if let Ok(v) = serde_json::from_value(value) { self.base_salary = v; }
                }
                _ => {} // ignore unknown fields
            }
        }
    }

    // <<< CUSTOM METHODS START >>>
    // <<< CUSTOM METHODS END >>>
}

impl super::Entity for Employee {
    type Id = Uuid;

    fn entity_id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "Employee"
    }
}

impl backbone_core::PersistentEntity for Employee {
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

impl backbone_orm::EntityRepoMeta for Employee {
    fn column_types() -> std::collections::HashMap<String, String> {
        let mut m = std::collections::HashMap::new();
        m.insert("id".to_string(), "uuid".to_string());
        m.insert("company_id".to_string(), "uuid".to_string());
        m.insert("user_id".to_string(), "uuid".to_string());
        m.insert("department_id".to_string(), "uuid".to_string());
        m.insert("employment_type".to_string(), "employment_type".to_string());
        m.insert("status".to_string(), "employee_status".to_string());
        m.insert("tax_status".to_string(), "tax_status".to_string());
        m
    }
    fn search_fields() -> &'static [&'static str] {
        &["employee_number", "first_name"]
    }
    fn company_field() -> Option<&'static str> {
        Some("company_id")
    }
}

/// Builder for Employee entity
///
/// Provides a fluent API for constructing Employee instances.
/// System fields (id, metadata, timestamps) are auto-initialized.
#[derive(Debug, Clone, Default)]
pub struct EmployeeBuilder {
    company_id: Option<Uuid>,
    employee_number: Option<String>,
    user_id: Option<Uuid>,
    department_id: Option<Uuid>,
    first_name: Option<String>,
    last_name: Option<String>,
    designation: Option<String>,
    employment_type: Option<EmploymentType>,
    date_of_joining: Option<DateTime<Utc>>,
    date_of_exit: Option<DateTime<Utc>>,
    status: Option<EmployeeStatus>,
    nik: Option<String>,
    npwp: Option<String>,
    tax_status: Option<TaxStatus>,
    bank_account_no: Option<String>,
    base_salary: Option<Decimal>,
}

impl EmployeeBuilder {
    /// Set the company_id field (required)
    pub fn company_id(mut self, value: Uuid) -> Self {
        self.company_id = Some(value);
        self
    }

    /// Set the employee_number field (required)
    pub fn employee_number(mut self, value: String) -> Self {
        self.employee_number = Some(value);
        self
    }

    /// Set the user_id field (optional)
    pub fn user_id(mut self, value: Uuid) -> Self {
        self.user_id = Some(value);
        self
    }

    /// Set the department_id field (optional)
    pub fn department_id(mut self, value: Uuid) -> Self {
        self.department_id = Some(value);
        self
    }

    /// Set the first_name field (required)
    pub fn first_name(mut self, value: String) -> Self {
        self.first_name = Some(value);
        self
    }

    /// Set the last_name field (optional)
    pub fn last_name(mut self, value: String) -> Self {
        self.last_name = Some(value);
        self
    }

    /// Set the designation field (optional)
    pub fn designation(mut self, value: String) -> Self {
        self.designation = Some(value);
        self
    }

    /// Set the employment_type field (default: `EmploymentType::default()`)
    pub fn employment_type(mut self, value: EmploymentType) -> Self {
        self.employment_type = Some(value);
        self
    }

    /// Set the date_of_joining field (required)
    pub fn date_of_joining(mut self, value: DateTime<Utc>) -> Self {
        self.date_of_joining = Some(value);
        self
    }

    /// Set the date_of_exit field (optional)
    pub fn date_of_exit(mut self, value: DateTime<Utc>) -> Self {
        self.date_of_exit = Some(value);
        self
    }

    /// Set the status field (default: `EmployeeStatus::default()`)
    pub fn status(mut self, value: EmployeeStatus) -> Self {
        self.status = Some(value);
        self
    }

    /// Set the nik field (optional)
    pub fn nik(mut self, value: String) -> Self {
        self.nik = Some(value);
        self
    }

    /// Set the npwp field (optional)
    pub fn npwp(mut self, value: String) -> Self {
        self.npwp = Some(value);
        self
    }

    /// Set the tax_status field (default: `TaxStatus::default()`)
    pub fn tax_status(mut self, value: TaxStatus) -> Self {
        self.tax_status = Some(value);
        self
    }

    /// Set the bank_account_no field (optional)
    pub fn bank_account_no(mut self, value: String) -> Self {
        self.bank_account_no = Some(value);
        self
    }

    /// Set the base_salary field (default: `Decimal::from(0)`)
    pub fn base_salary(mut self, value: Decimal) -> Self {
        self.base_salary = Some(value);
        self
    }

    /// Build the Employee entity
    ///
    /// Returns Err if any required field without a default is missing.
    pub fn build(self) -> Result<Employee, String> {
        let company_id = self.company_id.ok_or_else(|| "company_id is required".to_string())?;
        let employee_number = self.employee_number.ok_or_else(|| "employee_number is required".to_string())?;
        let first_name = self.first_name.ok_or_else(|| "first_name is required".to_string())?;
        let date_of_joining = self.date_of_joining.ok_or_else(|| "date_of_joining is required".to_string())?;

        Ok(Employee {
            id: Uuid::new_v4(),
            company_id,
            employee_number,
            user_id: self.user_id,
            department_id: self.department_id,
            first_name,
            last_name: self.last_name,
            designation: self.designation,
            employment_type: self.employment_type.unwrap_or(EmploymentType::default()),
            date_of_joining,
            date_of_exit: self.date_of_exit,
            status: self.status.unwrap_or(EmployeeStatus::default()),
            nik: self.nik,
            npwp: self.npwp,
            tax_status: self.tax_status.unwrap_or(TaxStatus::default()),
            bank_account_no: self.bank_account_no,
            base_salary: self.base_salary.unwrap_or(Decimal::from(0)),
            metadata: AuditMetadata::default(),
        })
    }
}
