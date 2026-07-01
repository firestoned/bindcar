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

#[cfg(test)]
mod validation_security_tests {
    use crate::records::{validate_record_name, validate_record_value};

    #[test]
    fn test_validate_record_value_rejects_control_chars_for_txt() {
        // TXT/CAA/SRV previously accepted "any non-empty string"; newline/CR/NUL
        // must now be rejected to prevent nsupdate command injection (B-2).
        for value in ["v=spf1\nupdate add evil", "a\rb", "x\0y"] {
            assert!(
                validate_record_value("TXT", value).is_err(),
                "expected TXT value {value:?} to be rejected"
            );
        }
    }

    #[test]
    fn test_validate_record_value_rejects_control_chars_for_caa_srv() {
        assert!(validate_record_value("CAA", "0 issue \"ca\"\nx").is_err());
        assert!(validate_record_value("SRV", "0 5 443 host.\ninject").is_err());
    }

    #[test]
    fn test_validate_record_value_accepts_clean_txt() {
        assert!(validate_record_value("TXT", "v=spf1 -all").is_ok());
        assert!(validate_record_value("CAA", "0 issue \"letsencrypt.org\"").is_ok());
    }

    #[test]
    fn test_validate_record_value_still_rejects_empty() {
        assert!(validate_record_value("TXT", "").is_err());
    }

    #[test]
    fn test_validate_record_name_rejects_control_chars() {
        for name in ["www\nupdate add evil", "a\rb", "x\0y"] {
            assert!(
                validate_record_name(name).is_err(),
                "expected name {name:?} to be rejected"
            );
        }
    }

    #[test]
    fn test_validate_record_name_accepts_clean_names() {
        for name in [
            "www",
            "@",
            "sub.example.com.",
            "_dmarc",
            "*",
            "*.wildcard",
            "_sip._tcp",
            "host-1",
        ] {
            assert!(
                validate_record_name(name).is_ok(),
                "expected name {name:?} to be accepted"
            );
        }
    }

    #[test]
    fn test_validate_record_name_rejects_zone_file_directives_without_control_chars() {
        // The record name is rendered at the START of a zone-file line, so a
        // value containing spaces / `$` / `;` can plant a `$INCLUDE`/`$GENERATE`
        // master-file directive even though it holds no control character.
        for name in [
            "$INCLUDE /etc/bind/rndc.key ;",
            "$GENERATE 1-16777215 host$",
            "www example",    // embedded space
            "a;comment",      // statement/comment metachar
            "name\"quoted\"", // quote
            "name(paren)",    // parens
            "$ORIGIN evil.",  // directive
            "",               // empty would emit a leading-space line
        ] {
            assert!(
                validate_record_name(name).is_err(),
                "expected name {name:?} to be rejected"
            );
        }
    }
}
