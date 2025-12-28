# Architecture Deep Dive

Deep dive into bindcar's internal architecture.

## Component Architecture

bindcar is built using Rust and the Axum web framework:

```mermaid
graph TB
    subgraph "HTTP Layer"
        HTTP[HTTP Server<br/>Axum]
        Router[Router<br/>Axum Routing]
    end

    subgraph "Middleware Layer"
        Auth[Authentication<br/>Middleware]
        Logging[Tracing<br/>Middleware]
    end

    subgraph "Application Layer"
        Handlers[Request Handlers]
        ZoneMgr[Zone Manager]
        RndcExec[RNDC Executor]
    end

    subgraph "Infrastructure Layer"
        Tokio[Tokio Runtime<br/>Async I/O]
        Process[Process Executor<br/>tokio::process]
        FileSystem[File System<br/>Zone Files]
    end

    HTTP --> Router
    Router --> Auth
    Auth --> Logging
    Logging --> Handlers
    Handlers --> ZoneMgr
    ZoneMgr --> RndcExec
    RndcExec --> Process
    ZoneMgr --> FileSystem
    Process --> FileSystem

    style HTTP fill:#e1f5ff
    style Tokio fill:#ffe1e1
    style FileSystem fill:#fff4e1
```

### Core Technologies

- **axum**: HTTP server and routing
- **tokio**: Async runtime and process execution
- **serde**: JSON serialization/deserialization
- **tracing**: Structured logging and observability

## Request Processing Flow

Complete request lifecycle from HTTP request to response:

```mermaid
sequenceDiagram
    participant Client
    participant Axum as Axum Router
    participant Auth as Auth Middleware
    participant Handler as Request Handler
    participant Validator as Validator
    participant ZoneMgr as Zone Manager
    participant FS as File System
    participant RNDC as RNDC Executor
    participant BIND9

    Client->>Axum: HTTP Request
    Axum->>Auth: Route to handler

    alt No Auth Token
        Auth-->>Client: 401 Unauthorized
    else Valid Token
        Auth->>Handler: Pass request
    end

    Handler->>Validator: Validate input

    alt Invalid Input
        Validator-->>Client: 400 Bad Request
    else Valid Input
        Validator->>ZoneMgr: Process request
    end

    ZoneMgr->>FS: Write/Read zone file
    ZoneMgr->>RNDC: Execute RNDC command
    RNDC->>BIND9: rndc addzone/delzone/reload

    alt RNDC Success
        BIND9-->>RNDC: Exit code 0
        RNDC-->>ZoneMgr: Success
        ZoneMgr-->>Handler: Success response
        Handler-->>Client: 200/201/204
    else RNDC Failure
        BIND9-->>RNDC: Exit code 1
        RNDC-->>ZoneMgr: Error
        ZoneMgr-->>Handler: Error response
        Handler-->>Client: 500 Internal Server Error
    end
```

## Code Structure

```mermaid
graph LR
    subgraph "src/"
        Main[main.rs<br/>Entry Point<br/>Server Setup]
        API[api.rs<br/>Routes & Handlers<br/>HTTP Endpoints]
        Auth[auth.rs<br/>Authentication<br/>Token Validation]
        RNDC[rndc.rs<br/>RNDC Integration<br/>Command Execution]
        Zones[zones.rs<br/>Zone Management<br/>File Operations]
        Models[models.rs<br/>Data Models<br/>Serde Types]
    end

    Main --> API
    API --> Auth
    API --> Zones
    Zones --> RNDC
    Zones --> Models
    API --> Models

    style Main fill:#e1f5ff
    style API fill:#ffe1e1
    style RNDC fill:#fff4e1
```

### Module Responsibilities

- **main.rs**: Application entry point, server configuration, dependency injection
- **api.rs**: HTTP route definitions, request/response handlers, OpenAPI specs
- **auth.rs**: Bearer token validation, Kubernetes ServiceAccount auth
- **rndc.rs**: RNDC command execution, process management, error handling
- **zones.rs**: Zone file generation, file I/O, zone lifecycle management
- **models.rs**: Request/response types, serialization, validation

## Async Processing Architecture

All I/O operations use async/await for non-blocking execution:

```mermaid
graph TB
    subgraph "Tokio Runtime"
        Scheduler[Task Scheduler]
        ThreadPool[Worker Thread Pool]
    end

    subgraph "Async Tasks"
        HTTPTask1[HTTP Handler 1]
        HTTPTask2[HTTP Handler 2]
        HTTPTask3[HTTP Handler 3]
        RNDCTask1[RNDC Execution 1]
        RNDCTask2[RNDC Execution 2]
        FileTask[File I/O Task]
    end

    Scheduler --> ThreadPool
    ThreadPool --> HTTPTask1
    ThreadPool --> HTTPTask2
    ThreadPool --> HTTPTask3
    ThreadPool --> RNDCTask1
    ThreadPool --> RNDCTask2
    ThreadPool --> FileTask

    HTTPTask1 -.->|await| RNDCTask1
    HTTPTask2 -.->|await| RNDCTask2
    HTTPTask3 -.->|await| FileTask

    style Scheduler fill:#e1f5ff
    style ThreadPool fill:#ffe1e1
```

### Benefits

- **Non-blocking HTTP handling**: Concurrent request processing
- **Async process execution**: Non-blocking RNDC command execution
- **Concurrent operations**: Multiple zones can be managed simultaneously
- **Efficient resource usage**: Small thread pool handles many concurrent operations

## Error Handling Strategy

Structured error types with proper HTTP status code mapping:

```mermaid
graph TD
    Request[Incoming Request] --> Validation{Validation}

    Validation -->|Invalid JSON| E400_1[400 Bad Request]
    Validation -->|Missing fields| E400_2[400 Bad Request]
    Validation -->|Invalid zone name| E400_3[400 Bad Request]

    Validation -->|Valid| Auth{Authentication}
    Auth -->|No token| E401[401 Unauthorized]
    Auth -->|Invalid token| E401

    Auth -->|Valid| Processing{Processing}
    Processing -->|Zone exists| E409[409 Conflict]
    Processing -->|Zone not found| E404[404 Not Found]
    Processing -->|RNDC error| E500[500 Internal Server Error]
    Processing -->|File I/O error| E500[500 Internal Error]
    Processing -->|Success| S200[200/201/204 Success]

    style E400_1 fill:#ffe1e1
    style E401 fill:#ffe1e1
    style E409 fill:#ffe1e1
    style E404 fill:#ffe1e1
    style E500 fill:#ffe1e1
    style E500 fill:#ffe1e1
    style S200 fill:#e1ffe1
```

### Error Type Mapping

| Error Type | HTTP Status | Use Case |
|------------|-------------|----------|
| ValidationError | 400 Bad Request | Invalid input data |
| AuthenticationError | 401 Unauthorized | Missing/invalid token |
| NotFoundError | 404 Not Found | Zone doesn't exist |
| ConflictError | 409 Conflict | Zone already exists |
| InternalError | 500 Internal Server Error | File I/O failures |
| RndcError | 500 Internal Server Error | RNDC command failures |

## Next Steps

- [RNDC Integration](./rndc-integration.md) - RNDC details
- [Contributing](./contributing.md) - Contribution guide
