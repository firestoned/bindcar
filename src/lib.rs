// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! bindcar - HTTP REST API for managing BIND9 zones via RNDC
//!
//! A lightweight library that provides programmatic control over BIND9 DNS zones
//! using the RNDC (Remote Name Daemon Control) protocol.
//!
//! # Features
//!
//! - Create, delete, and manage BIND9 zones dynamically
//! - Execute RNDC commands asynchronously
//! - Zone file generation and management
//! - Shared request/response types for API operations
//! - Authentication support (Bearer tokens and Kubernetes ServiceAccounts)
//! - Prometheus metrics integration
//!
//! # Usage
//!
//! This crate can be used as both a library and a standalone binary:
//!
//! ## As a Library
//!
//! ### Using RNDC Executor
//!
//! ```rust,no_run
//! use bindcar::RndcExecutor;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let executor = RndcExecutor::new(
//!         "127.0.0.1:953".to_string(),
//!         "sha256".to_string(),
//!         "dGVzdC1zZWNyZXQtaGVyZQ==".to_string(), // base64 encoded secret
//!     )?;
//!
//!     // Execute RNDC commands
//!     let status = executor.status().await?;
//!     println!("BIND9 Status: {}", status);
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Using Shared Types (for API clients)
//!
//! ```rust
//! use bindcar::{CreateZoneRequest, ZoneConfig, SoaRecord, DnsRecord};
//! use std::collections::HashMap;
//!
//! // Create nameserver glue records
//! let mut ns_ips = HashMap::new();
//! ns_ips.insert("ns1.example.com.".to_string(), "192.0.2.10".to_string());
//!
//! // Create a zone creation request
//! let request = CreateZoneRequest {
//!     zone_name: "example.com".to_string(),
//!     zone_type: "master".to_string(),
//!     zone_config: ZoneConfig {
//!         ttl: 3600,
//!         soa: SoaRecord {
//!             primary_ns: "ns1.example.com.".to_string(),
//!             admin_email: "admin.example.com.".to_string(),
//!             serial: 2025010101,
//!             refresh: 3600,
//!             retry: 600,
//!             expire: 604800,
//!             negative_ttl: 86400,
//!         },
//!         name_servers: vec!["ns1.example.com.".to_string()],
//!         name_server_ips: ns_ips,
//!         records: vec![
//!             DnsRecord {
//!                 name: "@".to_string(),
//!                 record_type: "A".to_string(),
//!                 value: "192.0.2.1".to_string(),
//!                 ttl: None,
//!                 priority: None,
//!             },
//!         ],
//!     },
//!     update_key_name: None,
//! };
//!
//! // Serialize to JSON for API requests
//! let json = serde_json::to_string(&request).unwrap();
//! ```
//!
//! ## As a Binary
//!
//! ```bash
//! cargo install bindcar
//! bindcar
//! ```
//!
//! ## Zone File Generation
//!
//! The `serial` field in SoaRecord will auto-generate in YYYYMMDD01 format if omitted from JSON.
//!
//! ```rust
//! use bindcar::{ZoneConfig, SoaRecord, DnsRecord};
//! use std::collections::HashMap;
//!
//! // Example JSON can omit serial for auto-generation:
//! let json = r#"{
//!   "ttl": 3600,
//!   "soa": {
//!     "primaryNs": "ns1.example.com.",
//!     "adminEmail": "admin.example.com."
//!   },
//!   "nameServers": ["ns1.example.com."],
//!   "nameServerIps": {
//!     "ns1.example.com.": "192.0.2.10"
//!   },
//!   "records": []
//! }"#;
//!
//! let zone_config: ZoneConfig = serde_json::from_str(json).unwrap();
//! let zone_file_content = zone_config.to_zone_file();
//! println!("{}", zone_file_content);
//! ```
//!
//! # Integration with Other Projects
//!
//! This library is designed to be used by other projects (like bindy) that need to
//! interact with the bindcar API. By importing this crate, you get:
//!
//! - Type-safe request/response structures
//! - Automatic JSON serialization/deserialization
//! - OpenAPI schema compatibility
//! - No need to maintain duplicate type definitions

// Re-export public modules
pub mod auth;
pub mod metrics;
pub mod middleware;
pub mod rndc;
pub mod types;
pub mod zones;

// Re-export commonly used types

// RNDC executor
pub use rndc::RndcExecutor;

// Error types
pub use types::{ApiError, AppState, ErrorResponse};

// Zone configuration types
pub use zones::{DnsRecord, SoaRecord, ZoneConfig};

// Request/Response types for API operations
pub use zones::{
    CreateZoneRequest, ServerStatusResponse, ZoneInfo, ZoneListResponse, ZoneResponse,
};

// RNDC configuration
pub use rndc::{parse_rndc_conf, RndcConfig};

// Test modules
#[cfg(test)]
mod auth_test;
#[cfg(test)]
mod metrics_test;
#[cfg(test)]
mod middleware_test;
#[cfg(test)]
mod rndc_test;
#[cfg(test)]
mod types_test;
#[cfg(test)]
mod zones_test;
