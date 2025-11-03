# Getting Started

## Prerequisites

- Rust 1.70 or higher
- PostgreSQL database
- Platform Network access
- TDX/SGX/SEV-SNP capable hardware (for production deployments)

## Installation

### Option 1: Docker Compose (Recommended)

```bash
docker-compose up --build
```

To run in background:
```bash
docker-compose up -d --build
```

To view logs:
```bash
docker-compose logs -f
```

### Option 2: Docker Direct

Build and run production:
```bash
docker build -t platform-api:prod --target platform-api .
docker run -p 3000:3000 -p 9090:9090 platform-api:prod
```

### Option 3: Local with Cargo

```bash
cargo run --release --bin platform-api-server
```

## Configuration

### Required Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgresql://user:pass@localhost/platform` |
| `STORAGE_ENCRYPTION_KEY` | Encryption key for storage (REQUIRED in production) | - |
| `JWT_SECRET` | JWT signing secret (REQUIRED in production) | - |
| `KBS_ENCRYPTION_KEY` | Key Broker Service encryption key (REQUIRED in production) | - |

### Optional Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level | `info` |
| `TEE_ENFORCED` | Enable TEE verification | `false` |
| `PORT` | HTTP server port | `3000` |
| `METRICS_PORT` | Prometheus metrics port | `9090` |
| `STORAGE_BACKEND` | Storage backend type | `postgres` |

## Quick Start

1. **Set required environment variables**:
   ```bash
   export STORAGE_ENCRYPTION_KEY="your-encryption-key"
   export JWT_SECRET="your-jwt-secret"
   export KBS_ENCRYPTION_KEY="your-kbs-key"
   export DATABASE_URL="postgresql://user:pass@localhost/platform"
   ```

2. **Start the API server**:
   ```bash
   cargo run --release --bin platform-api-server
   ```

3. **Verify it's running**:
   ```bash
   curl http://localhost:3000/version
   curl http://localhost:9090/metrics
   ```

## Next Steps

- Read the [Architecture](architecture.md) documentation
- Learn about [Security](security.md) features
- Check the [API Reference](api-reference.md)

