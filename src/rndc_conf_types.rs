// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! RNDC configuration data types
//!
//! This module defines the data structures for representing BIND9 rndc.conf files.
//!
//! # Examples
//!
//! ```rust
//! use bindcar::rndc_conf_types::{RndcConfFile, KeyBlock, OptionsBlock};
//!
//! let mut conf = RndcConfFile::new();
//! conf.keys.insert(
//!     "rndc-key".to_string(),
//!     KeyBlock::new(
//!         "rndc-key".to_string(),
//!         "hmac-sha256".to_string(),
//!         "dGVzdC1zZWNyZXQ=".to_string(),
//!     ),
//! );
//! conf.options.default_key = Some("rndc-key".to_string());
//!
//! let serialized = conf.to_conf_file();
//! ```

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;

/// Complete RNDC configuration file
#[derive(Debug, Clone, PartialEq)]
pub struct RndcConfFile {
    /// Named key blocks
    pub keys: HashMap<String, KeyBlock>,

    /// Server blocks indexed by address
    pub servers: HashMap<String, ServerBlock>,

    /// Global options
    pub options: OptionsBlock,

    /// Included files (resolved paths)
    pub includes: Vec<PathBuf>,
}

impl RndcConfFile {
    /// Create a new empty RNDC configuration
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            servers: HashMap::new(),
            options: OptionsBlock::default(),
            includes: Vec::new(),
        }
    }

    /// Get the default key (from options.default_key)
    pub fn get_default_key(&self) -> Option<&KeyBlock> {
        let key_name = self.options.default_key.as_ref()?;
        self.keys.get(key_name)
    }

    /// Get the default server address (from options.default_server)
    pub fn get_default_server(&self) -> Option<String> {
        self.options.default_server.clone()
    }

    /// Serialize to rndc.conf format
    pub fn to_conf_file(&self) -> String {
        let mut output = String::new();

        // Write includes
        for include_path in &self.includes {
            output.push_str(&format!("include \"{}\";\n", include_path.display()));
        }

        // Write keys
        for (name, key) in &self.keys {
            output.push_str(&format!("\nkey \"{}\" {}\n", name, key.to_conf_block()));
        }

        // Write servers
        for (addr, server) in &self.servers {
            output.push_str(&format!("\nserver {} {}\n", addr, server.to_conf_block()));
        }

        // Write options
        if !self.options.is_empty() {
            output.push_str(&format!("\noptions {}\n", self.options.to_conf_block()));
        }

        output
    }
}

impl Default for RndcConfFile {
    fn default() -> Self {
        Self::new()
    }
}

/// Key block: authentication credentials
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBlock {
    pub name: String,
    pub algorithm: String,
    pub secret: String,
}

impl KeyBlock {
    /// Create a new key block
    pub fn new(name: String, algorithm: String, secret: String) -> Self {
        Self {
            name,
            algorithm,
            secret,
        }
    }

    /// Serialize to RNDC-compatible key block
    ///
    /// Returns the configuration in the format:
    /// ```text
    /// {
    ///     algorithm hmac-sha256;
    ///     secret "dGVzdC1zZWNyZXQ=";
    /// };
    /// ```
    pub fn to_conf_block(&self) -> String {
        format!(
            "{{\n    algorithm {};\n    secret \"{}\";\n}};",
            self.algorithm, self.secret
        )
    }
}

/// Server block: server-specific configuration
#[derive(Debug, Clone, PartialEq)]
pub struct ServerBlock {
    pub address: ServerAddress,
    pub key: Option<String>,
    pub port: Option<u16>,
    pub addresses: Option<Vec<IpAddr>>,
}

impl ServerBlock {
    /// Create a new server block
    pub fn new(address: ServerAddress) -> Self {
        Self {
            address,
            key: None,
            port: None,
            addresses: None,
        }
    }

    /// Serialize to RNDC-compatible server block
    ///
    /// Returns the configuration in the format:
    /// ```text
    /// {
    ///     key "keyname";
    ///     port 953;
    /// };
    /// ```
    pub fn to_conf_block(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref key) = self.key {
            parts.push(format!("    key \"{}\";", key));
        }

        if let Some(port) = self.port {
            parts.push(format!("    port {};", port));
        }

        if let Some(ref addrs) = self.addresses {
            let addr_list = addrs
                .iter()
                .map(|ip| format!("        {};", ip))
                .collect::<Vec<_>>()
                .join("\n");
            parts.push(format!("    addresses {{\n{}\n    }};", addr_list));
        }

        if parts.is_empty() {
            "{ };".to_string()
        } else {
            format!("{{\n{}\n}};", parts.join("\n"))
        }
    }
}

/// Server address: hostname or IP address
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerAddress {
    Hostname(String),
    IpAddr(IpAddr),
}

impl ServerAddress {
    /// Parse a server address from a string
    pub fn parse(s: &str) -> Self {
        match s.parse::<IpAddr>() {
            Ok(addr) => ServerAddress::IpAddr(addr),
            Err(_) => ServerAddress::Hostname(s.to_string()),
        }
    }
}

impl std::fmt::Display for ServerAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerAddress::Hostname(h) => write!(f, "{}", h),
            ServerAddress::IpAddr(ip) => write!(f, "{}", ip),
        }
    }
}

/// Options block: global configuration
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptionsBlock {
    pub default_server: Option<String>,
    pub default_key: Option<String>,
    pub default_port: Option<u16>,
}

impl OptionsBlock {
    /// Create a new options block
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the options block is empty
    pub fn is_empty(&self) -> bool {
        self.default_server.is_none() && self.default_key.is_none() && self.default_port.is_none()
    }

    /// Serialize to RNDC-compatible options block
    ///
    /// Returns the configuration in the format:
    /// ```text
    /// {
    ///     default-server localhost;
    ///     default-key "rndc-key";
    ///     default-port 953;
    /// };
    /// ```
    pub fn to_conf_block(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref server) = self.default_server {
            parts.push(format!("    default-server {};", server));
        }

        if let Some(ref key) = self.default_key {
            parts.push(format!("    default-key \"{}\";", key));
        }

        if let Some(port) = self.default_port {
            parts.push(format!("    default-port {};", port));
        }

        if parts.is_empty() {
            "{ };".to_string()
        } else {
            format!("{{\n{}\n}};", parts.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_block_serialization() {
        let key = KeyBlock::new(
            "rndc-key".to_string(),
            "hmac-sha256".to_string(),
            "dGVzdC1zZWNyZXQ=".to_string(),
        );

        let serialized = key.to_conf_block();
        assert!(serialized.contains("algorithm hmac-sha256;"));
        assert!(serialized.contains("secret \"dGVzdC1zZWNyZXQ=\";"));
    }

    #[test]
    fn test_server_block_serialization() {
        let mut server = ServerBlock::new(ServerAddress::Hostname("localhost".to_string()));
        server.key = Some("rndc-key".to_string());
        server.port = Some(953);

        let serialized = server.to_conf_block();
        assert!(serialized.contains("key \"rndc-key\";"));
        assert!(serialized.contains("port 953;"));
    }

    #[test]
    fn test_server_block_empty() {
        let server = ServerBlock::new(ServerAddress::Hostname("localhost".to_string()));
        let serialized = server.to_conf_block();
        assert_eq!(serialized, "{ };");
    }

    #[test]
    fn test_options_block_serialization() {
        let mut options = OptionsBlock::new();
        options.default_server = Some("localhost".to_string());
        options.default_key = Some("rndc-key".to_string());
        options.default_port = Some(953);

        let serialized = options.to_conf_block();
        assert!(serialized.contains("default-server localhost;"));
        assert!(serialized.contains("default-key \"rndc-key\";"));
        assert!(serialized.contains("default-port 953;"));
    }

    #[test]
    fn test_options_block_empty() {
        let options = OptionsBlock::new();
        assert!(options.is_empty());
        assert_eq!(options.to_conf_block(), "{ };");
    }

    #[test]
    fn test_rndc_conf_file_serialization() {
        let mut conf = RndcConfFile::new();

        conf.keys.insert(
            "rndc-key".to_string(),
            KeyBlock::new(
                "rndc-key".to_string(),
                "hmac-sha256".to_string(),
                "dGVzdC1zZWNyZXQ=".to_string(),
            ),
        );

        conf.options.default_key = Some("rndc-key".to_string());
        conf.options.default_server = Some("localhost".to_string());

        let serialized = conf.to_conf_file();
        assert!(serialized.contains("key \"rndc-key\""));
        assert!(serialized.contains("algorithm hmac-sha256;"));
        assert!(serialized.contains("default-key \"rndc-key\";"));
        assert!(serialized.contains("default-server localhost;"));
    }

    #[test]
    fn test_get_default_key() {
        let mut conf = RndcConfFile::new();

        conf.keys.insert(
            "rndc-key".to_string(),
            KeyBlock::new(
                "rndc-key".to_string(),
                "hmac-sha256".to_string(),
                "dGVzdC1zZWNyZXQ=".to_string(),
            ),
        );

        conf.options.default_key = Some("rndc-key".to_string());

        let key = conf.get_default_key().unwrap();
        assert_eq!(key.name, "rndc-key");
        assert_eq!(key.algorithm, "hmac-sha256");
    }

    #[test]
    fn test_get_default_server() {
        let mut conf = RndcConfFile::new();
        conf.options.default_server = Some("localhost".to_string());

        assert_eq!(conf.get_default_server(), Some("localhost".to_string()));
    }

    #[test]
    fn test_server_address_parse() {
        let ip_addr = ServerAddress::parse("127.0.0.1");
        assert!(matches!(ip_addr, ServerAddress::IpAddr(_)));

        let hostname = ServerAddress::parse("localhost");
        assert!(matches!(hostname, ServerAddress::Hostname(_)));
    }

    #[test]
    fn test_server_address_display() {
        let hostname = ServerAddress::Hostname("localhost".to_string());
        assert_eq!(format!("{}", hostname), "localhost");

        let ip = ServerAddress::IpAddr("127.0.0.1".parse().unwrap());
        assert_eq!(format!("{}", ip), "127.0.0.1");
    }

    #[test]
    fn test_include_serialization() {
        let mut conf = RndcConfFile::new();
        conf.includes.push(PathBuf::from("/etc/bind/rndc.key"));

        let serialized = conf.to_conf_file();
        assert!(serialized.contains("include \"/etc/bind/rndc.key\";"));
    }

    #[test]
    fn test_server_with_addresses() {
        let mut server = ServerBlock::new(ServerAddress::Hostname("localhost".to_string()));
        server.addresses = Some(vec![
            "192.168.1.1".parse().unwrap(),
            "192.168.1.2".parse().unwrap(),
        ]);

        let serialized = server.to_conf_block();
        assert!(serialized.contains("addresses {"));
        assert!(serialized.contains("192.168.1.1;"));
        assert!(serialized.contains("192.168.1.2;"));
    }
}
