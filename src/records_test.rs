// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Tests for record management handlers

#[cfg(test)]
mod tests {
    use crate::records::{AddRecordRequest, RemoveRecordRequest, UpdateRecordRequest};

    #[test]
    fn test_add_record_request_serialization() {
        let request = AddRecordRequest {
            name: "www".to_string(),
            record_type: "A".to_string(),
            value: "192.0.2.1".to_string(),
            ttl: 3600,
            priority: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"name\":\"www\""));
        assert!(json.contains("\"type\":\"A\""));
        assert!(json.contains("\"value\":\"192.0.2.1\""));
        assert!(json.contains("\"ttl\":3600"));
    }

    #[test]
    fn test_remove_record_request_serialization() {
        let request = RemoveRecordRequest {
            name: "www".to_string(),
            record_type: "A".to_string(),
            value: Some("192.0.2.1".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"name\":\"www\""));
        assert!(json.contains("\"type\":\"A\""));
        assert!(json.contains("\"value\":\"192.0.2.1\""));
    }

    #[test]
    fn test_remove_record_request_without_value() {
        let request = RemoveRecordRequest {
            name: "www".to_string(),
            record_type: "A".to_string(),
            value: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"name\":\"www\""));
        assert!(json.contains("\"type\":\"A\""));
        // Value should be omitted when None
        assert!(!json.contains("\"value\""));
    }

    #[test]
    fn test_update_record_request_serialization() {
        let request = UpdateRecordRequest {
            name: "www".to_string(),
            record_type: "A".to_string(),
            current_value: "192.0.2.1".to_string(),
            new_value: "192.0.2.2".to_string(),
            ttl: 7200,
            priority: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"name\":\"www\""));
        assert!(json.contains("\"type\":\"A\""));
        assert!(json.contains("\"currentValue\":\"192.0.2.1\""));
        assert!(json.contains("\"newValue\":\"192.0.2.2\""));
        assert!(json.contains("\"ttl\":7200"));
    }

    #[test]
    fn test_add_record_request_with_priority() {
        let request = AddRecordRequest {
            name: "@".to_string(),
            record_type: "MX".to_string(),
            value: "mail.example.com.".to_string(),
            ttl: 3600,
            priority: Some(10),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"priority\":10"));
    }

    // Note: Full integration tests with API handlers require a running BIND9 server
    // and are better suited for integration test suites. These unit tests validate
    // request/response serialization and type correctness.
}
