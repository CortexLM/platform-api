# API Reference

## Overview

Platform API provides REST endpoints and WebSocket connections for managing challenges, jobs, and validators.

## Endpoints

### Health Check

```http
GET /health
```

Returns API health status.

### Version

```http
GET /version
```

Returns API version information.

### Challenges

#### List Challenges

```http
GET /api/challenges
```

Returns list of registered challenges.

#### Get Challenge

```http
GET /api/challenges/{challenge_id}
```

Returns challenge details.

#### Create Challenge

```http
POST /api/challenges
Content-Type: application/json

{
  "name": "challenge-name",
  "repository": "https://github.com/org/repo",
  "docker_image": "challenge:tag"
}
```

### Jobs

#### List Jobs

```http
GET /api/jobs?status=pending
```

Returns list of jobs with optional status filter.

#### Get Job

```http
GET /api/jobs/{job_id}
```

Returns job details.

### Validators

#### List Validators

```http
GET /api/validators
```

Returns list of connected validators.

#### Validator Status

```http
GET /api/validators/{validator_hotkey}/status
```

Returns validator status and challenge assignments.

## WebSocket

### Validator Connection

```rust
ws://api.platform.network/ws/validator
```

Connects validators to Platform API for job distribution and status updates.

### Challenge Connection

```rust
ws://api.platform.network/ws/challenge/{challenge_id}
```

Connects challenges for ORM bridge and lifecycle management.

## Authentication

### Validator Authentication

Validators authenticate using:
- Hotkey signature verification
- TDX attestation for secure channels

### Challenge Authentication

Challenges authenticate using:
- TDX attestation
- Challenge-specific credentials

## Rate Limiting

API endpoints may be rate-limited. Check response headers for rate limit information.

## Error Responses

All errors follow this format:

```json
{
  "error": "Error message",
  "code": "ERROR_CODE"
}
```

Common error codes:
- `400` - Bad Request
- `401` - Unauthorized
- `404` - Not Found
- `500` - Internal Server Error

