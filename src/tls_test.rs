// Copyright (c) 2025 Erick Bourgeois, firestoned
// SPDX-License-Identifier: MIT

//! Unit tests for TLS transport configuration.

use crate::tls::{
    build_client_verifier, build_server_config, resolve_tls_settings, scheme_for, TlsError,
};
use std::io::Write;
use tempfile::NamedTempFile;

/// A self-signed leaf used only by these tests. Not a secret: the matching key
/// below is a throwaway generated for the test suite and is valid for
/// `localhost`/`127.0.0.1` only.
const TEST_CERT_PEM: &str = r#"
-----BEGIN CERTIFICATE-----
MIIDLTCCAhWgAwIBAgIUYlBwjtBn2jUsXSiLDl8uru9SePEwDQYJKoZIhvcNAQEL
BQAwFzEVMBMGA1UEAwwMYmluZGNhci10ZXN0MCAXDTI2MDkxODIwMzkxMVoYDzIx
MjYwODI1MjAzOTExWjAXMRUwEwYDVQQDDAxiaW5kY2FyLXRlc3QwggEiMA0GCSqG
SIb3DQEBAQUAA4IBDwAwggEKAoIBAQDKBH/Lqcxi39At4FfIZiUvOQE9R3pfatIX
wIqaFTdq6+p5yry4WHEgawfY2/8fMrn58rmgScz6eosLDzY5sbhhF66ZQuwPioum
VGNNbgpoFk9uEg8FgE93/Wk3R1iVSxPhOxEZImZtS85xnO7tY11i3jbvD3IEMLo7
+RL8JRb315llcPnBoHk9Bd5E6+9JZactc97+4Naa557f8t0lfZPAIEkP/Xa2dPZD
JR33I/ILztmCQVHH4MC24aor/Hc68sCBJdB7wkju7RBEgb34Y1xGcmu8+3uWJiKZ
1sr/C4t8pMtT8PdBQtW9J18dNnA77e759GYgqAURXsInBYY0jPmVAgMBAAGjbzBt
MB0GA1UdDgQWBBSAY4t9Q2Fn/QCZFDUKqBrcQbcSTDAfBgNVHSMEGDAWgBSAY4t9
Q2Fn/QCZFDUKqBrcQbcSTDAPBgNVHRMBAf8EBTADAQH/MBoGA1UdEQQTMBGCCWxv
Y2FsaG9zdIcEfwAAATANBgkqhkiG9w0BAQsFAAOCAQEAgs1gNP86qJ2+qA+kFVmC
oCfeBjjLcmcluPe4LgrQLEpHaGXekor+lrWL61LIshiPmFyGlJGkxTxZ9KkJ+Riy
cBZp+N71c806jNxnhMXdgO5a3u8u/MZ2qQSegv0WWdinAILG0YDVXJowvUkgz8d5
bLlNwFCTeTNV+7DKDu3ep9AO4+sTllfNrpjr/luyidwFhXCm1YVq3OjtO1fKxVt3
O+dCO35nvWQQd4vhj6eB5z8TApOAfWlX5EeI/sUpg1u9SeBsS5WXnQUxxAcaPMeI
fk/4mvgHc04/OIG4vsHhkYPphAd3Hshgu3eCunA4sKfkaSfMXf+g7Cb+IIfXvF+K
gQ==
-----END CERTIFICATE-----
"#;

const TEST_KEY_PEM: &str = r#"
-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDKBH/Lqcxi39At
4FfIZiUvOQE9R3pfatIXwIqaFTdq6+p5yry4WHEgawfY2/8fMrn58rmgScz6eosL
DzY5sbhhF66ZQuwPioumVGNNbgpoFk9uEg8FgE93/Wk3R1iVSxPhOxEZImZtS85x
nO7tY11i3jbvD3IEMLo7+RL8JRb315llcPnBoHk9Bd5E6+9JZactc97+4Naa557f
8t0lfZPAIEkP/Xa2dPZDJR33I/ILztmCQVHH4MC24aor/Hc68sCBJdB7wkju7RBE
gb34Y1xGcmu8+3uWJiKZ1sr/C4t8pMtT8PdBQtW9J18dNnA77e759GYgqAURXsIn
BYY0jPmVAgMBAAECggEACAvCOnTxBf/oa7TwUIDFx5BOqbq+3fpqk080D5guuw+m
wnw+CsP2Mkm9znfpb+9DE8U3rXGjCPFE6zpxrD4uzypa5hHE3+2VseWeIUeNTieT
uRkqoTGpRWjBywjVapGXh7RX8jDtJ9QJs3uCBjebg/eNWo7thjGZ1g+nUoTFQGoZ
etTfpCOD25NXqA2DYLIILkOydVFrXNB9FqSrW4oHSYQFgYz0x3dXCEKS1iBXsvE4
0/R1M3KjCnxSkHPrUq5xcBONbyyYhSIflcqZzGYRdMD9ytEnT5eot8IC8xqZqi1c
tQJBOdj8LsQIWw5yUsAoC9VM46cUJkoP53Oxhpbj1QKBgQD2wzQXKOBXc1rKcxb1
GwgtSuQSGCYSPs5vvrBohEcRYqTHlS5trEp/rtJPByugeTRXwdt3iU4WZ56X2Yrf
yRbwDHUw0+2q8OlDwouone6na+Kt0neEQjSxsOuMFwIQbp6Gh3bEYxxE6lb0VeaO
GqrYHMLLC4n86ZYT4rer6yRXKwKBgQDRlH32NfjNFGwg7FXT2hO3BEDWKIMIKq6p
om5LuxtH0Z1C/OlJnHwXuDMKqAXe0IRxRuRWLvmZE0glpUVrkK19oMlE8RdHzzBQ
aN/mQcXyqNn9w2NfWnpdcyK6HmVmvEjfzVgakuy3kZJrsRZuk358I54+RGm/NYN1
cxZD1C+SPwKBgGN9KmhYA7Nef/F124CxCGfydOfSsq7SgbrOACPziQ+6XMNXI2P2
fgbivko8kttdYrwrHcghJMmlt2xzuikl00ivTSSFnaI5BWNbcaFnI4x+0+LPI37A
jqxBr4ZI1H05jFKjFUBy0Tf731kdtRoAKHd/iQ4CNf0xVF/qHbGD2aAHAoGAFEs2
r0KmpuUVW1LHNM5nHk+xH4uotH+9jfuGhprFl3y6p6PpyxD2Cy3w81U1zE+Qo49j
yNyfmqz9TXflcvb9da6+Dojx4igz23VsSNWRn1+uTB5BXxhZxPbDJBaRZxNQUyuF
Hn2fol7cOMVbELYDh23DgvAI9VTvN84/F65SNO0CgYEA8cuq90EA9ukrJi1WeOvW
j98sQo2BNwbdpamkC31Yr3zil7HggEXZ+tECZuD+xyCTyGkoQS+d4ySgrI+BODUk
9b8NxiChHKWorY9MshkPm644523fyucMmLgn6pDqVQBJKZuAQcU+BH46nigSIasZ
/b5394tMg6S5LTRMHYglTl0=
-----END PRIVATE KEY-----
"#;

fn pem_file(contents: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().expect("create temp pem");
    f.write_all(contents.as_bytes()).expect("write pem");
    f.flush().expect("flush pem");
    f
}

fn path_of(f: &NamedTempFile) -> String {
    f.path().to_str().expect("utf-8 temp path").to_string()
}

// ---------------------------------------------------------------------------
// resolve_tls_settings — pure resolution, no filesystem access
// ---------------------------------------------------------------------------

/// With neither cert nor key configured, bindcar serves plaintext. This is the
/// default and must stay backward compatible.
#[test]
fn test_resolve_tls_settings_none_configured_is_plaintext() {
    assert!(resolve_tls_settings(None, None, None)
        .expect("no TLS config is valid")
        .is_none());
}

/// Cert + key together enable TLS.
#[test]
fn test_resolve_tls_settings_cert_and_key_enables_tls() {
    let settings = resolve_tls_settings(Some("/c.pem".into()), Some("/k.pem".into()), None)
        .expect("cert+key is valid")
        .expect("TLS should be enabled");

    assert_eq!(settings.cert_path, "/c.pem");
    assert_eq!(settings.key_path, "/k.pem");
    assert!(settings.client_ca_path.is_none());
    assert!(!settings.requires_client_auth());
}

/// A half-configured listener must fail closed. Silently falling back to
/// plaintext when only one half is set would serve the API in the clear on a
/// deployment whose operator believes TLS is on.
#[test]
fn test_resolve_tls_settings_half_configured_is_rejected() {
    let cert_only = resolve_tls_settings(Some("/c.pem".into()), None, None);
    assert!(matches!(cert_only, Err(TlsError::IncompleteKeyPair { .. })));

    let key_only = resolve_tls_settings(None, Some("/k.pem".into()), None);
    assert!(matches!(key_only, Err(TlsError::IncompleteKeyPair { .. })));
}

/// A client CA without a server key pair cannot be honoured — mTLS requires a
/// TLS listener. Fail rather than silently ignore the CA.
#[test]
fn test_resolve_tls_settings_client_ca_without_tls_is_rejected() {
    let r = resolve_tls_settings(None, None, Some("/ca.pem".into()));
    assert!(matches!(r, Err(TlsError::ClientCaWithoutTls)));
}

/// Cert + key + client CA enables mTLS.
#[test]
fn test_resolve_tls_settings_client_ca_enables_mtls() {
    let settings = resolve_tls_settings(
        Some("/c.pem".into()),
        Some("/k.pem".into()),
        Some("/ca.pem".into()),
    )
    .expect("cert+key+ca is valid")
    .expect("TLS should be enabled");

    assert_eq!(settings.client_ca_path.as_deref(), Some("/ca.pem"));
    assert!(settings.requires_client_auth());
}

/// Empty strings are treated as unset, so an unset env var that expands to ""
/// does not half-configure the listener.
#[test]
fn test_resolve_tls_settings_empty_strings_are_unset() {
    assert!(
        resolve_tls_settings(Some(String::new()), Some(String::new()), None)
            .expect("empty means unset")
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// build_server_config — loads PEM material from disk
// ---------------------------------------------------------------------------

/// The happy path: a valid cert/key pair produces a usable rustls ServerConfig.
#[test]
fn test_build_server_config_loads_valid_pem() {
    let cert = pem_file(TEST_CERT_PEM);
    let key = pem_file(TEST_KEY_PEM);
    let settings = resolve_tls_settings(Some(path_of(&cert)), Some(path_of(&key)), None)
        .expect("valid")
        .expect("tls on");

    let config = build_server_config(&settings).expect("valid pem should build a ServerConfig");
    assert!(
        !config.alpn_protocols.is_empty(),
        "ALPN must advertise HTTP"
    );
}

/// A missing certificate file is a startup error, not a silent fallback.
#[test]
fn test_build_server_config_rejects_missing_cert() {
    let key = pem_file(TEST_KEY_PEM);
    let settings = resolve_tls_settings(
        Some("/nonexistent-bindcar-cert.pem".into()),
        Some(path_of(&key)),
        None,
    )
    .expect("valid")
    .expect("tls on");

    assert!(matches!(
        build_server_config(&settings),
        Err(TlsError::CertRead { .. })
    ));
}

/// A malformed PEM body is rejected rather than producing an empty chain.
#[test]
fn test_build_server_config_rejects_malformed_cert() {
    let cert = pem_file("not a certificate\n");
    let key = pem_file(TEST_KEY_PEM);
    let settings = resolve_tls_settings(Some(path_of(&cert)), Some(path_of(&key)), None)
        .expect("valid")
        .expect("tls on");

    assert!(build_server_config(&settings).is_err());
}

/// A key file holding no private key is rejected.
#[test]
fn test_build_server_config_rejects_missing_key_material() {
    let cert = pem_file(TEST_CERT_PEM);
    let key = pem_file("-----BEGIN CERTIFICATE-----\nnope\n-----END CERTIFICATE-----\n");
    let settings = resolve_tls_settings(Some(path_of(&cert)), Some(path_of(&key)), None)
        .expect("valid")
        .expect("tls on");

    assert!(matches!(
        build_server_config(&settings),
        Err(TlsError::KeyRead { .. }) | Err(TlsError::KeyParse { .. })
    ));
}

/// mTLS: a client CA bundle that parses produces a config demanding client certs.
#[test]
fn test_build_server_config_with_client_ca_requires_client_auth() {
    let cert = pem_file(TEST_CERT_PEM);
    let key = pem_file(TEST_KEY_PEM);
    let ca = pem_file(TEST_CERT_PEM);
    let settings = resolve_tls_settings(
        Some(path_of(&cert)),
        Some(path_of(&key)),
        Some(path_of(&ca)),
    )
    .expect("valid")
    .expect("tls on");

    build_server_config(&settings).expect("mTLS config should build");

    // Assert the security property itself: the verifier must *require* a client
    // certificate, not merely offer to check one if presented.
    let verifier = build_client_verifier(&path_of(&ca)).expect("verifier should build");
    assert!(
        verifier.client_auth_mandatory(),
        "a configured client CA must make client certs mandatory"
    );
}

/// An unreadable client CA path fails startup.
#[test]
fn test_build_server_config_rejects_missing_client_ca() {
    let cert = pem_file(TEST_CERT_PEM);
    let key = pem_file(TEST_KEY_PEM);
    let settings = resolve_tls_settings(
        Some(path_of(&cert)),
        Some(path_of(&key)),
        Some("/nonexistent-bindcar-ca.pem".into()),
    )
    .expect("valid")
    .expect("tls on");

    assert!(matches!(
        build_server_config(&settings),
        Err(TlsError::ClientCaRead { .. })
    ));
}

// ---------------------------------------------------------------------------
// scheme reporting
// ---------------------------------------------------------------------------

/// The startup banner must advertise the scheme actually in use — the Swagger
/// URL previously hardcoded `http://` regardless of transport.
#[test]
fn test_scheme_reflects_transport() {
    let tls = resolve_tls_settings(Some("/c.pem".into()), Some("/k.pem".into()), None)
        .expect("valid")
        .expect("tls on");
    assert_eq!(scheme_for(Some(&tls)), "https");
    assert_eq!(scheme_for(None), "http");
}
