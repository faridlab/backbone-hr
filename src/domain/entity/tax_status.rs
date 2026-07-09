use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "tax_status", rename_all = "snake_case")]
pub enum TaxStatus {
    Tk0,
    Tk1,
    Tk2,
    Tk3,
    K0,
    K1,
    K2,
    K3,
}

impl std::fmt::Display for TaxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tk0 => write!(f, "tk0"),
            Self::Tk1 => write!(f, "tk1"),
            Self::Tk2 => write!(f, "tk2"),
            Self::Tk3 => write!(f, "tk3"),
            Self::K0 => write!(f, "k0"),
            Self::K1 => write!(f, "k1"),
            Self::K2 => write!(f, "k2"),
            Self::K3 => write!(f, "k3"),
        }
    }
}

impl FromStr for TaxStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tk0" => Ok(Self::Tk0),
            "tk1" => Ok(Self::Tk1),
            "tk2" => Ok(Self::Tk2),
            "tk3" => Ok(Self::Tk3),
            "k0" => Ok(Self::K0),
            "k1" => Ok(Self::K1),
            "k2" => Ok(Self::K2),
            "k3" => Ok(Self::K3),
            _ => Err(format!("Unknown TaxStatus variant: {}", s)),
        }
    }
}

impl Default for TaxStatus {
    fn default() -> Self {
        Self::Tk0
    }
}
