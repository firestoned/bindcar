# Security

Security considerations for bindcar deployments.

## Overview

This section covers security best practices for bindcar and BIND9.

## Authentication

- Bearer token authentication
- Kubernetes ServiceAccount tokens
- Token rotation strategies

## Transport Security

- TLS for the REST API (`BIND_TLS_CERT` / `BIND_TLS_KEY`)
- Mutual TLS via a client CA bundle (`BIND_TLS_CLIENT_CA`)
- Certificate provisioning with cert-manager

Every request carries a privileged credential — a ServiceAccount token or
`BIND_API_TOKEN`. Authentication proves *who* the caller is; only TLS keeps that
credential confidential in transit. bindcar serves plaintext by default and
warns at startup when it does so on a non-loopback address.

See [TLS Transport](./tls.md).

## Access Control

- Network policies
- RBAC permissions
- API endpoint protection

## BIND9 Security

- RNDC key management
- Zone transfer restrictions
- Query access controls

## Next Steps

- [TLS Transport](./tls.md) - Serving the API over TLS and mutual TLS
- [Authentication & Authorization](./auth.md) - Detailed auth configuration
- [Access Control](./access-control.md) - Access control patterns
