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

#[cfg(test)]
mod injection_tests {
    use crate::nsupdate::reject_injection_chars;

    #[test]
    fn test_reject_injection_chars_blocks_newline_and_cr_and_nul() {
        assert!(reject_injection_chars("value", "1.2.3.4\nupdate add evil").is_err());
        assert!(reject_injection_chars("name", "www\r\nsend").is_err());
        assert!(reject_injection_chars("zone", "example.com\0").is_err());
    }

    #[test]
    fn test_reject_injection_chars_allows_clean_values() {
        assert!(reject_injection_chars("value", "192.0.2.1").is_ok());
        assert!(reject_injection_chars("name", "www.example.com.").is_ok());
        assert!(reject_injection_chars("zone", "example.com").is_ok());
    }
}

/// B-7: the TSIG secret must never appear in the nsupdate argument vector —
/// argv is world-readable via /proc/<pid>/cmdline. The key is passed via a
/// 0600 temp key file (-k) instead.
#[cfg(test)]
mod tsig_keyfile_tests {
    use crate::nsupdate::{build_nsupdate_args, build_tsig_key_file_content, NsupdateExecutor};

    const SECRET: &str = "dGVzdC1zZWNyZXQ=";

    fn executor_with_tsig() -> NsupdateExecutor {
        NsupdateExecutor::new(
            "127.0.0.1".to_string(),
            53,
            Some("update-key".to_string()),
            Some("HMAC-SHA256".to_string()),
            Some(SECRET.to_string()),
        )
        .unwrap()
    }

    #[test]
    fn test_args_never_contain_secret() {
        let executor = executor_with_tsig();
        let keyfile = executor.create_tsig_key_file().unwrap().unwrap();
        let args = build_nsupdate_args(false, Some(keyfile.path()));

        for arg in &args {
            let s = arg.to_string_lossy();
            assert!(
                !s.contains(SECRET),
                "argv must not contain the TSIG secret, found in {s:?}"
            );
        }
        // -y must be gone entirely
        assert!(args.iter().all(|a| a != "-y"));
        // -k <path> must be present
        assert!(args.iter().any(|a| a == "-k"));
    }

    #[test]
    fn test_args_without_tsig_or_tcp_are_empty() {
        assert!(build_nsupdate_args(false, None).is_empty());
    }

    #[test]
    fn test_args_include_tcp_flag() {
        let args = build_nsupdate_args(true, None);
        assert_eq!(args, vec![std::ffi::OsString::from("-v")]);
    }

    #[test]
    fn test_keyfile_is_created_with_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let executor = executor_with_tsig();
        let keyfile = executor.create_tsig_key_file().unwrap().unwrap();
        let mode = keyfile.path().metadata().unwrap().permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o600,
            "TSIG key file must be readable by owner only"
        );
    }

    #[test]
    fn test_keyfile_content_and_cleanup() {
        let executor = executor_with_tsig();
        let keyfile = executor.create_tsig_key_file().unwrap().unwrap();
        let path = keyfile.path().to_path_buf();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("key \"update-key\""));
        assert!(content.contains("algorithm hmac-sha256;"));
        assert!(content.contains(&format!("secret \"{SECRET}\";")));

        // Dropping the guard unlinks the file.
        drop(keyfile);
        assert!(!path.exists(), "key file must be removed after use");
    }

    #[test]
    fn test_no_keyfile_without_tsig() {
        let executor =
            NsupdateExecutor::new("127.0.0.1".to_string(), 53, None, None, None).unwrap();
        assert!(executor.create_tsig_key_file().unwrap().is_none());
    }

    #[test]
    fn test_key_file_content_normalizes_algorithm() {
        let content = build_tsig_key_file_content("k", "SHA256", "YWJj").unwrap();
        assert!(content.contains("algorithm hmac-sha256;"));
        let content = build_tsig_key_file_content("k", "hmac-sha512", "YWJj").unwrap();
        assert!(content.contains("algorithm hmac-sha512;"));
    }

    #[test]
    fn test_key_file_content_rejects_bad_inputs() {
        // Key name breaking out of the quoted literal
        assert!(build_tsig_key_file_content("k\"; };", "sha256", "YWJj").is_err());
        assert!(build_tsig_key_file_content("k name", "sha256", "YWJj").is_err());
        assert!(build_tsig_key_file_content("", "sha256", "YWJj").is_err());
        // Unknown algorithm
        assert!(build_tsig_key_file_content("k", "rot13", "YWJj").is_err());
        // Non-base64 secret (quote breakout)
        assert!(build_tsig_key_file_content("k", "sha256", "abc\"; };").is_err());
        assert!(build_tsig_key_file_content("k", "sha256", "").is_err());
    }
}
