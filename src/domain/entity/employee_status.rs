use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "employee_status", rename_all = "snake_case")]
pub enum EmployeeStatus {
    Active,
    Resigned,
    Terminated,
}

impl std::fmt::Display for EmployeeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Resigned => write!(f, "resigned"),
            Self::Terminated => write!(f, "terminated"),
        }
    }
}

impl FromStr for EmployeeStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "resigned" => Ok(Self::Resigned),
            "terminated" => Ok(Self::Terminated),
            _ => Err(format!("Unknown EmployeeStatus variant: {}", s)),
        }
    }
}

impl Default for EmployeeStatus {
    fn default() -> Self {
        Self::Active
    }
}
