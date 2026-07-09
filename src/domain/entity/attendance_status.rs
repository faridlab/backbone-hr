use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "attendance_status", rename_all = "snake_case")]
pub enum AttendanceStatus {
    Present,
    Absent,
    HalfDay,
    OnLeave,
    Holiday,
}

impl std::fmt::Display for AttendanceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Present => write!(f, "present"),
            Self::Absent => write!(f, "absent"),
            Self::HalfDay => write!(f, "half_day"),
            Self::OnLeave => write!(f, "on_leave"),
            Self::Holiday => write!(f, "holiday"),
        }
    }
}

impl FromStr for AttendanceStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "present" => Ok(Self::Present),
            "absent" => Ok(Self::Absent),
            "half_day" => Ok(Self::HalfDay),
            "on_leave" => Ok(Self::OnLeave),
            "holiday" => Ok(Self::Holiday),
            _ => Err(format!("Unknown AttendanceStatus variant: {}", s)),
        }
    }
}

impl Default for AttendanceStatus {
    fn default() -> Self {
        Self::Present
    }
}
