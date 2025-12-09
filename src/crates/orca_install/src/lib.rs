//! Orca Install - Application initialization and configuration tool
//!
//! This crate provides tools to initialize orca and aco applications from
//! YAML configuration files, seeding databases and generating TOML configs.

pub mod config;
pub mod database;
pub mod installer;
pub mod schema;

pub use installer::Installer;
pub use schema::{AcoInstallConfig, OrcaInstallConfig};
