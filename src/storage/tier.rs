use serde::{Deserialize, Serialize};
use std::fmt;

/// Storage tier representing the performance/cost characteristics of a pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageTier {
    Hot,
    Warm,
    Cold,
}

impl StorageTier {
    /// Map an S3 storage class header to a storage tier.
    pub fn from_s3_storage_class(class: &str) -> Self {
        match class.to_uppercase().as_str() {
            "STANDARD" | "STANDARD_IA" | "REDUCED_REDUNDANCY" | "INTELLIGENT_TIERING" | "ONEZONE_IA"
            | "EXPRESS_ONEZONE" => StorageTier::Hot,
            "GLACIER_IR" | "DEEP_ARCHIVE_IA" => StorageTier::Warm,
            "GLACIER" | "DEEP_ARCHIVE" | "SNOW" => StorageTier::Cold,
            _ => StorageTier::Hot,
        }
    }

    /// Map a storage tier back to an S3 storage class name.
    pub fn to_s3_storage_class(self) -> &'static str {
        match self {
            StorageTier::Hot => "STANDARD",
            StorageTier::Warm => "STANDARD_IA",
            StorageTier::Cold => "GLACIER",
        }
    }

    /// Parse from a configuration string (case-insensitive).
    pub fn from_str_lower(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "hot" => StorageTier::Hot,
            "warm" => StorageTier::Warm,
            "cold" => StorageTier::Cold,
            _ => StorageTier::Hot,
        }
    }
}

impl fmt::Display for StorageTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageTier::Hot => write!(f, "HOT"),
            StorageTier::Warm => write!(f, "WARM"),
            StorageTier::Cold => write!(f, "COLD"),
        }
    }
}

impl std::str::FromStr for StorageTier {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "HOT" => Ok(StorageTier::Hot),
            "WARM" => Ok(StorageTier::Warm),
            "COLD" => Ok(StorageTier::Cold),
            _ => Err(format!("Unknown storage tier: {}", s)),
        }
    }
}
