# API Design

## Design Principles

bindcar's API is designed with the following principles:

1. **RESTful** - Follows REST conventions for resource management
2. **Simple** - Easy to understand and use
3. **Consistent** - Predictable request/response patterns
4. **Secure** - Authentication required for all operations
5. **Observable** - Comprehensive logging and health checks

## API Structure

```mermaid
graph TD
    Root["/api/v1"] --> Health["/health"]
    Root --> Ready["/ready"]
    Root --> Server["/server"]
    Root --> Zones["/zones"]

    Server --> Status["/status"]

    Zones --> Create["POST /"]
    Zones --> List["GET /"]
    Zones --> Zone["/:name"]

    Zone --> Get["GET"]
    Zone --> Delete["DELETE"]
    Zone --> Reload["/reload"]
    Zone --> ZStatus["/status"]
    Zone --> Freeze["/freeze"]
    Zone --> Thaw["/thaw"]
    Zone --> Notify["/notify"]

    style Health fill:#c8e6c9
    style Ready fill:#c8e6c9
    style Create fill:#bbdefb
    style Delete fill:#ffcdd2
```

## REST Resource Model

### Resources

| Resource | HTTP Methods | Description |
|----------|--------------|-------------|
| `/health` | GET | Service health status |
| `/ready` | GET | Service readiness status |
| `/server/status` | GET | BIND9 server status |
| `/zones` | GET, POST | Zone collection |
| `/zones/{name}` | GET, DELETE | Individual zone |
| `/zones/{name}/reload` | POST | Zone reload action |
| `/zones/{name}/status` | GET | Zone status query |
| `/zones/{name}/freeze` | POST | Zone freeze action |
| `/zones/{name}/thaw` | POST | Zone thaw action |
| `/zones/{name}/notify` | POST | Zone notify action |

### HTTP Method Usage

```mermaid
graph LR
    GET[GET] --> Read[Read Resources]
    POST[POST] --> Create[Create or Action]
    DELETE[DELETE] --> Remove[Delete Resources]
    
    Read -.->|Idempotent| Safe[Safe Operation]
    Create -.->|Non-Idempotent| Change[State Change]
    Remove -.->|Idempotent| Change
    
    style GET fill:#c8e6c9
    style POST fill:#bbdefb
    style DELETE fill:#ffcdd2
```

## Request/Response Format

### Standard Request

All requests follow this pattern:

```mermaid
graph LR
    Method[HTTP Method] --> URL[URL Path]
    URL --> Headers[Headers]
    Headers --> Body[Request Body]
    
    Headers --> Auth[Authorization: Bearer token]
    Headers --> Content[Content-Type: application/json]
    
    Body --> JSON[JSON Payload]
    
    style Auth fill:#fff3e0
    style JSON fill:#e1f5fe
```

### Standard Response

```mermaid
graph LR
    Status[HTTP Status Code] --> Headers[Response Headers]
    Headers --> Body[Response Body]
    
    Headers --> Content[Content-Type: application/json]
    
    Body --> Success{Success?}
    Success -->|Yes| Data[Data Object]
    Success -->|No| Error[Error Object]
    
    Data --> Fields[success, message, details]
    Error --> EFields[error, message]
    
    style Success fill:#e8f5e9
    style Error fill:#ffebee
```

## Response Schemas

### Success Response

```json
{
  "success": true,
  "message": "Operation completed successfully",
  "details": "Additional information"
}
```

### Error Response

```json
{
  "error": "Error type",
  "message": "Human-readable error message"
}
```

## Status Code Strategy

```mermaid
graph TD
    Request[API Request] --> Process{Processing}
    
    Process -->|Success| S2xx[2xx Success]
    Process -->|Client Error| E4xx[4xx Client Error]
    Process -->|Server Error| E5xx[5xx Server Error]
    
    S2xx --> S200[200 OK<br/>Successful GET/POST/DELETE]
    S2xx --> S201[201 Created<br/>Zone Created]
    
    E4xx --> E400[400 Bad Request<br/>Invalid Input]
    E4xx --> E401[401 Unauthorized<br/>Missing/Invalid Token]
    E4xx --> E404[404 Not Found<br/>Zone Doesn't Exist]
    
    E5xx --> E500[500 Internal Error<br/>Server Failure]
    E5xx --> E500[500 Internal Server Error<br/>RNDC Command Failed]
    E5xx --> E503[503 Service Unavailable<br/>Not Ready]
    
    style S200 fill:#c8e6c9
    style S201 fill:#c8e6c9
    style E400 fill:#ffebee
    style E401 fill:#ffebee
    style E404 fill:#ffebee
    style E500 fill:#ffebee
    style E500 fill:#ffebee
    style E503 fill:#ffebee
```

## Authentication Flow

```mermaid
sequenceDiagram
    participant Client
    participant K8s as Kubernetes
    participant API as bindcar API
    participant Handler
    
    Client->>K8s: Get ServiceAccount Token
    K8s-->>Client: Bearer Token
    Client->>API: Request + Authorization: Bearer <token>
    API->>API: Extract Token
    
    alt No Authorization Header
        API-->>Client: 401 + {"error": "Missing Authorization header"}
    else Invalid Format
        API-->>Client: 401 + {"error": "Invalid Authorization header format"}
    else Empty Token
        API-->>Client: 401 + {"error": "Empty token"}
    else Valid Token
        API->>Handler: Process Request
        Handler-->>API: Response
        API-->>Client: 200 + Response Data
    end
```

## Versioning Strategy

### URL Versioning

All endpoints include `/v1/` in the path:

```
/api/v1/zones
```

This allows for future API versions without breaking existing clients:

```
/api/v2/zones  (future)
```

### Version Migration Path

```mermaid
graph LR
    V1[API v1] --> Support1[Supported]
    V2[API v2] -.-> Future[Future Release]
    V2 -.-> Deprecate[Deprecate v1]
    Deprecate -.-> Remove[Remove v1]
    
    Support1 --> Docs1[v1 Documentation]
    Future --> Docs2[v2 Documentation]
    
    style V1 fill:#c8e6c9
    style V2 fill:#e1f5fe
```

## Content Negotiation

Currently, bindcar only supports JSON:

```http
Content-Type: application/json
Accept: application/json
```

Future versions may support additional formats:

```mermaid
graph TB
    Request[Request] --> Format{Accept Header}
    Format -->|application/json| JSON[JSON Response]
    Format -->|application/yaml| YAML[YAML Response]
    Format -->|*/*| Default[Default JSON]
    
    style JSON fill:#c8e6c9
    style YAML fill:#e1f5fe
    style Default fill:#fff9c4
```

## Error Handling Philosophy

```mermaid
graph TD
    Error[Error Occurs] --> Type{Error Type}
    
    Type -->|Validation| Client[Client Error 4xx]
    Type -->|Auth| Client
    Type -->|Not Found| Client
    
    Type -->|File I/O| Server[Server Error 5xx]
    Type -->|RNDC Failed| Gateway[Bad Gateway 500]
    Type -->|Unexpected| Server
    
    Client --> Log1[Log Warning]
    Server --> Log2[Log Error]
    Gateway --> Log3[Log Error]
    
    Log1 --> Response1[Return Error Response]
    Log2 --> Response1
    Log3 --> Response1
    
    Response1 --> Include[Include:<br/>- Error type<br/>- Human message<br/>- Status code]
    
    style Client fill:#fff3e0
    style Server fill:#ffebee
    style Gateway fill:#ffebee
```

## API Evolution Guidelines

### Adding New Endpoints

✅ **Allowed** (Backward Compatible):
- Adding new endpoints
- Adding optional fields to requests
- Adding fields to responses

❌ **Not Allowed** (Breaking Changes):
- Removing endpoints
- Removing response fields
- Making optional fields required
- Changing field types

### Breaking Change Management

```mermaid
graph LR
    Change[Breaking Change Needed] --> Version[Create New API Version]
    Version --> Parallel[Run Both Versions]
    Parallel --> Deprecate[Deprecate Old Version]
    Deprecate --> Wait[Wait Period<br/>6-12 months]
    Wait --> Remove[Remove Old Version]
    
    style Change fill:#ffebee
    style Version fill:#fff3e0
    style Remove fill:#ffcdd2
```

## OpenAPI / Swagger

bindcar provides interactive API documentation via Swagger UI:

```mermaid
graph TD
    Source["Rust Source Code"] --> Annotations["utoipa Annotations"]
    Annotations --> Generator["OpenAPI Generator"]
    Generator --> Spec["OpenAPI 3.0 Spec"]

    Spec --> JSON["/api/v1/openapi.json"]
    Spec --> UI["Swagger UI"]

    UI --> Interactive["/api/v1/docs"]

    style Annotations fill:#e1f5fe
    style Interactive fill:#c8e6c9
```

Access the interactive documentation:

```bash
# OpenAPI JSON specification
curl http://localhost:8080/api/v1/openapi.json

# Interactive Swagger UI
open http://localhost:8080/api/v1/docs
```

## Rate Limiting (Future)

While not currently implemented, future versions may include rate limiting:

```mermaid
graph TD
    Request[API Request] --> Check{Rate Limit?}
    Check -->|Under Limit| Process[Process Request]
    Check -->|Over Limit| Reject[429 Too Many Requests]
    
    Process --> Update[Update Counter]
    Reject --> Header[X-RateLimit-* Headers]
    
    style Reject fill:#ffebee
```
