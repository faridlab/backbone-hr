use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "employment_type", rename_all = "snake_case")]
pub enum EmploymentType {
    Permanent,
    Contract,
    Probation,
    Intern,
}

impl std::fmt::Display for EmploymentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Permanent => write!(f, "permanent"),
            Self::Contract => write!(f, "contract"),
            Self::Probation => write!(f, "probation"),
            Self::Intern => write!(f, "intern"),
        }
    }
}

impl FromStr for EmploymentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "permanent" => Ok(Self::Permanent),
            "contract" => Ok(Self::Contract),
            "probation" => Ok(Self::Probation),
            "intern" => Ok(Self::Intern),
            _ => Err(format!("Unknown EmploymentType variant: {}", s)),
        }
    }
}

impl Default for EmploymentType {
    fn default() -> Self {
        Self::Permanent
    }
}
