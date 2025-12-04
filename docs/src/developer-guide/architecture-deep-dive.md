# Architecture Deep Dive

Deep dive into bindcar's internal architecture.

## Component Architecture

bindcar is built using Rust and the Axum web framework:

- **axum**: HTTP server and routing
- **tokio**: Async runtime
- **serde**: JSON serialization
- **tracing**: Structured logging

## Request Processing

```
HTTP Request
  ↓
Axum Router
  ↓
Authentication Middleware
  ↓
Request Handler
  ↓
RNDC Executor
  ↓
Response
```

## Code Structure

```
src/
├── main.rs           # Entry point
├── api.rs            # API routes and handlers
├── auth.rs           # Authentication
├── rndc.rs           # RNDC integration
├── zones.rs          # Zone management
└── models.rs         # Data models
```

## Async Processing

All I/O operations use async/await:

- Non-blocking HTTP handling
- Async process execution for RNDC
- Concurrent request processing

## Error Handling

Structured error types with proper HTTP status mapping:

- Validation errors → 400
- Auth errors → 401
- Conflict errors → 409
- RNDC errors → 502

## Next Steps

- [RNDC Integration](./rndc-integration.md) - RNDC details
- [Contributing](./contributing.md) - Contribution guide
