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

// ---------------------------------------------------------------------------
// Hot-reload (roadmap 06)
// ---------------------------------------------------------------------------

use crate::tls::{fingerprint, TlsReloader, DEFAULT_RELOAD_INTERVAL_SECS};

/// A second, distinct key pair so a reload has something to change *to*.
/// Same shape as the pair above, different subject (CN=bindcar-test-renewed).
const TEST_CERT2_PEM: &str = r#"
-----BEGIN CERTIFICATE-----
MIIDYTCCAkmgAwIBAgIUAbFsxSX+EfNsRF7+sjjYtrlt5gkwDQYJKoZIhvcNAQEL
BQAwHzEdMBsGA1UEAwwUYmluZGNhci10ZXN0LXJlbmV3ZWQwIBcNMjYwOTE5MDMz
NzAzWhgPMjEyNjA4MjYwMzM3MDNaMB8xHTAbBgNVBAMMFGJpbmRjYXItdGVzdC1y
ZW5ld2VkMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA+XX7RSfbCH3o
WfwRXu7WYiw1PDYkgoOGdTNcJdVMB3nluBSYEcmDjankW0Jd5c1iqGZPKGRRjzf3
gonoMDZjTtEfayBUPAv4XEHhqGe7GoEtDVjRZkEfgK72ZqXLqeXHrVljmN6qPVtJ
unLOfPwNRvUHEwyUBEs45jvbJf+eSgsfkv7By8KBzw1KHLHZo2HW90lTMItxDhC0
MMshLRsb1pCgKK0+QIcfsd2Shxx+Ur/rWxec7I+kGAjxfoQ4wUaDYq+7E2IBb3TT
M3DAfvn18sEu8HsC/oB1EMRK5rMuAO1+OTNqAlzMdLgdQpISOlGCKituAxc00AAY
tX3MCHMufwIDAQABo4GSMIGPMB0GA1UdDgQWBBQdqNPm5UQJPw5KQRRDxe+qN+h2
3jAfBgNVHSMEGDAWgBQdqNPm5UQJPw5KQRRDxe+qN+h23jAMBgNVHRMBAf8EAjAA
MA4GA1UdDwEB/wQEAwIFoDATBgNVHSUEDDAKBggrBgEFBQcDATAaBgNVHREEEzAR
gglsb2NhbGhvc3SHBH8AAAEwDQYJKoZIhvcNAQELBQADggEBAOH2xO24zDuMmr4P
b0X1p4qzIu6mJNSRBA1T/GUV1kSw/q9MOV2IGyMNYaYVgVNHqmG/aJfhp9UXSwlD
J0X0zKQ+ye9v3wQK7svYnXxyv+jT3pJxZdGxNCScw45nRkG4XKHxXUqjYkc+mH15
W0O7aG08dIgQ4ZwvW7hJOsu/ZmYeM5j6aq86Ce9Y2LtH1kcIyReDw3S/zKa5WXh5
5OsD3mObdQ+m7vdADdeRk7wVx/cd0zsR8knLxKHe407S0HsO+05/q0JysmXKOcnU
DPasRAbqRRy2ClQnqlthSxEiChqx9t9UQ0ivkrRFlo6U1sFtmgsp2zi6/IK+YWmD
bMPAGS8=
-----END CERTIFICATE-----
"#;

const TEST_KEY2_PEM: &str = r#"
-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQD5dftFJ9sIfehZ
/BFe7tZiLDU8NiSCg4Z1M1wl1UwHeeW4FJgRyYONqeRbQl3lzWKoZk8oZFGPN/eC
iegwNmNO0R9rIFQ8C/hcQeGoZ7sagS0NWNFmQR+ArvZmpcup5cetWWOY3qo9W0m6
cs58/A1G9QcTDJQESzjmO9sl/55KCx+S/sHLwoHPDUocsdmjYdb3SVMwi3EOELQw
yyEtGxvWkKAorT5Ahx+x3ZKHHH5Sv+tbF5zsj6QYCPF+hDjBRoNir7sTYgFvdNMz
cMB++fXywS7wewL+gHUQxErmsy4A7X45M2oCXMx0uB1CkhI6UYIqK24DFzTQABi1
fcwIcy5/AgMBAAECggEAYI0zByWxZ3x/8VAYExPC4zV0F01nXSJ16KfLyxLevegG
qvRBlWTW534xlca+nAKd5ErQ6XPGg3WodRxWQ07RqgBTtj1JjQIfCuou4mTfrJcB
rnBJf9fFzyMo2DrkdGosmiIGY/UOk/fgterYk9RkeSm+JrfQFEdfCvFw1Si2Bba7
zGBHkQVrbLAgjcm/BYjmd+nELkaaICdebbQWSBjp7PVVt49rUBUbqp6Arhue8C2O
X7qOO32n/vmWV0PB97mp4Cku8fT8UkaKwPXsqyrlbxu9tLPtsUBN8jO8JBliPQ5M
KFAo/Muq0LelSKSyRDdpN2W/Wz3MkmnQQlobCZmzoQKBgQD/feYc5XSHprRDTg03
hYlt4Yp/2JjVC5mZxLkmDQNYkKIoBKkmP8gZWhk/A/mhf26ivqG/Dwe+SVPbuORl
FPI4y5Lo2/ZnXBvh8gX0xuh4lKL6WGll7LuhQgKNjliIQ6MDCNOUv03NaAQtsyd3
jy5hakAzqkg+8J7S2h6mHEHfIQKBgQD59QL3VOsARxBIZ2eHzDVU5Zog43FSM3Lx
vR9eZdrayOnqdDBkC2EfQ4uDR1hxZLRPamhab5wtuElBD1qMyLKjvRhMdbjBfS7q
ypJd8UlryVvL5ovcAQz9zqtPjRJd7/MaiuU2OcB0qw3ExoGZyUqqcBaEVGgUdKIg
DdH0tbN5nwKBgBQDutkcqIpP5uM25BYrYd63wm/NefuGkxvWq2JttotjmTBlXRLg
AD7sLfofx5h9MR+Sq30aIlMnz2fxDgNVJryIRhPz11O1hYGnwguw4VlA25uc/XS8
nN4/G5AXTJwwID8Gm/yVF/U1Zs5lsHvPPaTn++uQNWSo2OhPqgL9R7PBAoGBAIa4
qxeZ2mu04a7UpPWJeDlA41jUndB7UHnAwHaFmXcQkRs/8pEJnRhXtItWWfIMIC8p
oAMWYuw1hq4dU2XMCpS8J6uWS7Vl/nKoKkmd8j+5MNPud/VlT5ZA3Q6sb3jYCoSE
1lPqvrNjOrGTeGjmGGtSrKA4Sjy0PGngaQhnIsvrAoGBANgXJTbMd6UZCApgbt5L
NlwUqr2aQ4R8k0d7dl3ggq5DqwBAYXXKRB7cyn3U4DR9nEYAFpWDRINcRmGtUQxl
BwrSK4G0FpKiaIIhDmYm/XKh5idveSwVDoeIYyWvq01pWaGNHJXuen3X0eMwJgrr
YZtJK243xDO804NQhAK0MU8C
-----END PRIVATE KEY-----
"#;

/// Write `contents` over an existing temp file, as a renewal would.
fn rewrite(f: &NamedTempFile, contents: &str) {
    std::fs::write(f.path(), contents).expect("rewrite pem");
}

/// The fingerprint must be stable across calls when nothing on disk changed —
/// otherwise every poll would trigger a needless rebuild.
#[test]
fn test_fingerprint_is_stable_when_files_are_unchanged() {
    let cert = pem_file(TEST_CERT_PEM);
    let key = pem_file(TEST_KEY_PEM);
    let s = resolve_tls_settings(Some(path_of(&cert)), Some(path_of(&key)), None)
        .expect("valid")
        .expect("tls on");

    let a = fingerprint(&s).expect("fingerprint");
    let b = fingerprint(&s).expect("fingerprint");
    assert_eq!(a, b, "fingerprint must not change on its own");
}

/// A renewed certificate must change the fingerprint, or a reload would never
/// be triggered.
#[test]
fn test_fingerprint_changes_when_certificate_is_replaced() {
    let cert = pem_file(TEST_CERT_PEM);
    let key = pem_file(TEST_KEY_PEM);
    let s = resolve_tls_settings(Some(path_of(&cert)), Some(path_of(&key)), None)
        .expect("valid")
        .expect("tls on");

    let before = fingerprint(&s).expect("fingerprint");
    rewrite(&cert, TEST_CERT2_PEM);
    rewrite(&key, TEST_KEY2_PEM);
    let after = fingerprint(&s).expect("fingerprint");

    assert_ne!(
        before, after,
        "replacing the key pair must change the digest"
    );
}

/// Changing only the client CA must also be noticed — mTLS trust is part of the
/// reloadable surface, not just the server certificate.
#[test]
fn test_fingerprint_covers_the_client_ca() {
    let cert = pem_file(TEST_CERT_PEM);
    let key = pem_file(TEST_KEY_PEM);
    let ca = pem_file(TEST_CERT_PEM);
    let s = resolve_tls_settings(
        Some(path_of(&cert)),
        Some(path_of(&key)),
        Some(path_of(&ca)),
    )
    .expect("valid")
    .expect("tls on");

    let before = fingerprint(&s).expect("fingerprint");
    rewrite(&ca, TEST_CERT2_PEM);
    let after = fingerprint(&s).expect("fingerprint");

    assert_ne!(before, after, "client CA must be part of the fingerprint");
}

/// A valid renewal swaps the live config.
#[test]
fn test_reloader_swaps_config_on_valid_renewal() {
    let cert = pem_file(TEST_CERT_PEM);
    let key = pem_file(TEST_KEY_PEM);
    let s = resolve_tls_settings(Some(path_of(&cert)), Some(path_of(&key)), None)
        .expect("valid")
        .expect("tls on");

    let reloader = TlsReloader::new(s).expect("initial config must build");
    let first = reloader.current();

    assert!(!reloader.reload_if_changed(), "no change yet");

    rewrite(&cert, TEST_CERT2_PEM);
    rewrite(&key, TEST_KEY2_PEM);

    assert!(
        reloader.reload_if_changed(),
        "a renewal should be picked up"
    );
    assert!(
        !std::sync::Arc::ptr_eq(&first, &reloader.current()),
        "the live config must actually have been replaced"
    );
}

/// The rule that matters most: a half-written or corrupt certificate must NOT
/// take the listener down. The previous config keeps serving and the next poll
/// retries.
#[test]
fn test_reloader_keeps_serving_when_new_material_is_broken() {
    let cert = pem_file(TEST_CERT_PEM);
    let key = pem_file(TEST_KEY_PEM);
    let s = resolve_tls_settings(Some(path_of(&cert)), Some(path_of(&key)), None)
        .expect("valid")
        .expect("tls on");

    let reloader = TlsReloader::new(s).expect("initial config must build");
    let good = reloader.current();

    // Simulate observing the file mid-write: truncated PEM.
    rewrite(&cert, "-----BEGIN CERTIFICATE-----\ntruncated");

    assert!(
        !reloader.reload_if_changed(),
        "a broken renewal must not report a successful swap"
    );
    assert!(
        std::sync::Arc::ptr_eq(&good, &reloader.current()),
        "the previously good config must still be live"
    );

    // And once the write completes, the next poll recovers without intervention.
    rewrite(&cert, TEST_CERT2_PEM);
    rewrite(&key, TEST_KEY2_PEM);
    assert!(reloader.reload_if_changed(), "next poll must recover");
}

/// A mismatched pair (new cert, stale key) is the classic non-atomic-write
/// hazard and must be treated as a failed reload, not a swap.
#[test]
fn test_reloader_rejects_mismatched_cert_and_key() {
    let cert = pem_file(TEST_CERT_PEM);
    let key = pem_file(TEST_KEY_PEM);
    let s = resolve_tls_settings(Some(path_of(&cert)), Some(path_of(&key)), None)
        .expect("valid")
        .expect("tls on");

    let reloader = TlsReloader::new(s).expect("initial config must build");
    let good = reloader.current();

    rewrite(&cert, TEST_CERT2_PEM); // new cert, key not yet replaced

    assert!(
        !reloader.reload_if_changed(),
        "mismatched pair must not swap"
    );
    assert!(std::sync::Arc::ptr_eq(&good, &reloader.current()));
}

/// An interval of zero disables reloading, restoring startup-only behaviour.
#[test]
fn test_reload_interval_zero_disables_reloading() {
    assert!(!TlsReloader::reloading_enabled(0));
    assert!(TlsReloader::reloading_enabled(1));
    assert!(TlsReloader::reloading_enabled(DEFAULT_RELOAD_INTERVAL_SECS));
}
