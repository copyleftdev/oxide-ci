//! Strongly-typed identifiers for domain entities.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident, $prefix:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }

            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            pub fn as_uuid(&self) -> &Uuid {
                &self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}_{}", $prefix, self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                let uuid_str = s.strip_prefix(concat!($prefix, "_")).unwrap_or(s);
                Ok(Self(Uuid::parse_str(uuid_str)?))
            }
        }
    };
}

define_id!(PipelineId, "pip");
define_id!(RunId, "run");
define_id!(AgentId, "agt");
define_id!(SecretId, "sec");
define_id!(CacheEntryId, "cch");
define_id!(ApprovalGateId, "apr");
define_id!(NotificationChannelId, "ntf");
define_id!(ArtifactId, "art");
define_id!(MatrixId, "mtx");
define_id!(JobId, "job");
define_id!(LicenseId, "lic");

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StageId(String);

impl StageId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StepId(String);

impl StepId {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StepId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_id_display() {
        let id = PipelineId::new();
        let s = id.to_string();
        assert!(s.starts_with("pip_"));
    }

    #[test]
    fn test_pipeline_id_parse() {
        let id = PipelineId::new();
        let s = id.to_string();
        let parsed: PipelineId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }
}
