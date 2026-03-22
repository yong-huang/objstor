//! ObjStor - S3-Compatible Object Storage Simulator
//!
//! A high-performance, S3-compatible object storage system written in Rust.

pub mod api;
pub mod auth;
pub mod config;
pub mod error;
pub mod logging;
pub mod metadata;
pub mod scheduler;
pub mod storage;
pub mod web;

pub use config::{Config, ServerConfig, StorageConfig, StoragePoolConfig};
pub use error::{Error, Result};
