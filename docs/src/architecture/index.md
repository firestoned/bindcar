# Architecture

## Overview

bindcar is designed as a sidecar container that runs alongside BIND9, providing a REST API for zone management. It acts as a bridge between Kubernetes/container orchestration and BIND9's rndc control interface.

## System Architecture

```mermaid
graph TB
    subgraph "Kubernetes Pod"
        subgraph "bindcar Container"
            API[HTTP API Server]
            Auth[Authentication Middleware]
            Handler[Request Handlers]
            RNDC[RNDC Executor]
        end
        
        subgraph "BIND9 Container"
            BIND[BIND9 Server]
            RNDCDaemon[rndc Daemon :953]
        end
        
        SharedVol[Shared Volume<br/>/var/cache/bind]
    end
    
    Client[API Client]
    K8s[Kubernetes ServiceAccount]
    
    Client -->|Bearer Token| API
    K8s -.->|Provides Token| Client
    API --> Auth
    Auth --> Handler
    Handler --> RNDC
    Handler -.->|Write Zone Files| SharedVol
    RNDC -->|rndc commands| RNDCDaemon
    RNDCDaemon -.->|Manages| BIND
    BIND -.->|Reads| SharedVol
    
    style API fill:#e1f5ff
    style BIND fill:#ffe1e1
    style SharedVol fill:#e8f5e9
```

## Component Breakdown

### HTTP API Server (Axum)

- Handles incoming HTTP requests
- Provides OpenAPI/Swagger documentation
- Implements health and readiness probes
- Routes requests to appropriate handlers

```mermaid
graph LR
    Request[HTTP Request] --> Router[Axum Router]
    Router --> Health[Health Endpoints]
    Router --> Zones[Zone Endpoints]
    Router --> Server[Server Endpoints]
    
    Health --> Response[JSON Response]
    Zones --> Response
    Server --> Response
```

### Authentication Middleware

- Validates Bearer tokens
- Integrates with Kubernetes ServiceAccount tokens
- Protects all endpoints except health checks

```mermaid
sequenceDiagram
    participant Client
    participant Auth as Auth Middleware
    participant Handler as Request Handler
    
    Client->>Auth: Request + Bearer Token
    Auth->>Auth: Extract Token
    alt No Token
        Auth-->>Client: 401 Unauthorized
    else Invalid Format
        Auth-->>Client: 401 Unauthorized
    else Empty Token
        Auth-->>Client: 401 Unauthorized
    else Valid Token
        Auth->>Handler: Forward Request
        Handler-->>Auth: Response
        Auth-->>Client: 200 OK
    end
```

### RNDC Executor

Executes rndc commands via the system binary.

```mermaid
graph TD
    A["RNDC Executor"] --> B["tokio::process::Command"]
    B --> C["/usr/sbin/rndc"]
    C --> D{"Exit Code"}
    D -->|"0"| E["Parse stdout"]
    D -->|"!=0"| F["Parse stderr"]
    E --> G["Return Success"]
    F --> H["Return Error"]
```

### Zone File Management

```mermaid
sequenceDiagram
    participant API
    participant FS as File System
    participant RNDC
    participant BIND9
    
    API->>API: Generate Zone File Content
    API->>FS: Write /var/cache/bind/zone.zone
    FS-->>API: Success
    API->>RNDC: rndc addzone example.com { ... }
    RNDC->>BIND9: Add zone configuration
    BIND9->>FS: Read zone file
    BIND9-->>RNDC: Zone loaded
    RNDC-->>API: Success
```

## Request Flow

### Create Zone Flow

```mermaid
sequenceDiagram
    autonumber
    participant Client
    participant Auth as Auth Middleware
    participant Handler as Zone Handler
    participant FS as File System
    participant RNDC as RNDC Executor
    participant BIND9
    
    Client->>Auth: POST /api/v1/zones
    Auth->>Auth: Validate Token
    Auth->>Handler: Authorized Request
    Handler->>Handler: Validate Zone Config
    Handler->>Handler: Generate Zone File
    Handler->>FS: Write zone.zone
    FS-->>Handler: File Written
    Handler->>RNDC: Execute addzone
    RNDC->>BIND9: rndc addzone
    BIND9-->>RNDC: Zone Loaded
    RNDC-->>Handler: Success
    Handler-->>Auth: 201 Created
    Auth-->>Client: JSON Response
```

### Delete Zone Flow

```mermaid
sequenceDiagram
    autonumber
    participant Client
    participant Handler as Zone Handler
    participant RNDC as RNDC Executor
    participant BIND9
    participant FS as File System
    
    Client->>Handler: DELETE /api/v1/zones/{name}
    Handler->>RNDC: Execute delzone
    RNDC->>BIND9: rndc delzone
    BIND9-->>RNDC: Zone Removed
    RNDC-->>Handler: Success
    Handler->>FS: Delete zone.zone
    FS-->>Handler: File Deleted
    Handler-->>Client: 200 OK
```

## Data Flow

### Zone Configuration to Zone File

```mermaid
graph LR
    JSON[JSON Config] --> Parser[Config Parser]
    Parser --> SOA[SOA Record]
    Parser --> NS[NS Records]
    Parser --> RR[Resource Records]
    
    SOA --> Builder[Zone File Builder]
    NS --> Builder
    RR --> Builder
    
    Builder --> File[Zone File]
    
    style JSON fill:#e3f2fd
    style File fill:#f3e5f5
```

## Deployment Architecture

### Standalone Docker

```mermaid
graph TB
    subgraph "Docker Host"
        subgraph "bind9 Container"
            BIND[BIND9]
        end
        
        subgraph "bindcar Container"
            API[bindcar API]
        end
        
        Vol[Docker Volume<br/>zones]
    end
    
    BIND -.->|Reads| Vol
    API -.->|Writes| Vol
    API -->|rndc| BIND
    
    Client[External Client] -->|:8080| API
```

### Kubernetes Sidecar

```mermaid
graph TB
    subgraph "Kubernetes Cluster"
        subgraph "Pod"
            subgraph "bind9 Container"
                BIND[BIND9<br/>:53]
            end
            
            subgraph "bindcar Container"
                API[bindcar<br/>:8080]
            end
            
            Vol[emptyDir Volume]
        end
        
        Service[ClusterIP Service<br/>dns-api]
        SA[ServiceAccount<br/>dns-operator]
    end
    
    BIND -.->|Reads| Vol
    API -.->|Writes| Vol
    API -->|rndc :953| BIND
    Service -->|Routes| API
    SA -.->|Provides Token| API
    
    Operator[External Operator] -->|Authenticated| Service
    
    style Service fill:#fff3e0
    style SA fill:#e8f5e9
```

## Security Model

```mermaid
graph TD
    A[API Request] --> B{Has Auth Header?}
    B -->|No| C[401 Unauthorized]
    B -->|Yes| D{Valid Format?}
    D -->|No| C
    D -->|Yes| E{Token Present?}
    E -->|No| C
    E -->|Yes| F[Process Request]
    
    F --> G{Input Validation}
    G -->|Invalid| H[400 Bad Request]
    G -->|Valid| I{Execute Operation}
    
    I --> J{Success?}
    J -->|Yes| K[200/201 Response]
    J -->|No| L{Error Type}
    L -->|RNDC Error| M[502 Bad Gateway]
    L -->|Not Found| N[404 Not Found]
    L -->|Other| O[500 Internal Error]
```

## Error Handling

```mermaid
graph TD
    Req[Request] --> Val{Validation}
    Val -->|Invalid Input| E400[400 Bad Request]
    Val -->|Valid| Auth{Authentication}
    Auth -->|Failed| E401[401 Unauthorized]
    Auth -->|Success| Proc[Process Request]
    
    Proc --> FS{File Operations}
    FS -->|Failed| E500[500 Internal Error]
    FS -->|Success| RNDC{RNDC Command}
    
    RNDC -->|Failed| E502[502 Bad Gateway]
    RNDC -->|Not Found| E404[404 Not Found]
    RNDC -->|Success| Success[200/201 Success]
    
    style E400 fill:#ffebee
    style E401 fill:#ffebee
    style E404 fill:#ffebee
    style E500 fill:#ffebee
    style E502 fill:#ffebee
    style Success fill:#e8f5e9
```

## Technology Stack

| Layer | Technology |
|-------|------------|
| HTTP Framework | Axum 0.8 |
| Async Runtime | Tokio |
| Serialization | Serde + JSON |
| Logging | Tracing + tracing-subscriber |
| API Documentation | utoipa + Swagger UI |
| RNDC Communication | System rndc binary |
| Container Runtime | Docker / containerd |
| Orchestration | Kubernetes (optional) |
