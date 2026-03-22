//! ObjStor - S3-Compatible Object Storage Simulator
//!
//! A high-performance, S3-compatible object storage system written in Rust.

pub mod config;
pub mod error;
pub mod storage;
pub mod scheduler;
pub mod metadata;
pub mod auth;
pub mod logging;
pub mod api;
pub mod web;

pub use error::{Error, Result};
pub use config::{Config, ServerConfig, StorageConfig, StoragePoolConfig};
