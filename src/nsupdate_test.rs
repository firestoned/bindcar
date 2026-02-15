// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Tests for nsupdate executor

#[cfg(test)]
mod tests {
    use crate::nsupdate::NsupdateExecutor;

    #[test]
    fn test_new_executor_with_full_tsig() {
        let executor = NsupdateExecutor::new(
            "127.0.0.1".to_string(),
            53,
            Some("update-key".to_string()),
            Some("HMAC-SHA256".to_string()),
            Some("dGVzdC1zZWNyZXQ=".to_string()),
        );

        assert!(executor.is_ok());
    }

    #[test]
    fn test_new_executor_without_tsig() {
        let executor = NsupdateExecutor::new("10.0.0.1".to_string(), 5353, None, None, None);

        assert!(executor.is_ok());
    }

    #[test]
    fn test_new_executor_custom_port() {
        let executor = NsupdateExecutor::new(
            "192.168.1.1".to_string(),
            8053,
            Some("key".to_string()),
            Some("HMAC-SHA512".to_string()),
            Some("c2VjcmV0".to_string()),
        );

        assert!(executor.is_ok());
    }

    // Note: We cannot easily test the execute() method and record operations
    // without a real BIND9 server, so these are integration tests that should
    // be run separately with a test environment.
    //
    // For unit testing, we validate the struct creation and error parsing logic,
    // which are in the main nsupdate.rs file.
}
