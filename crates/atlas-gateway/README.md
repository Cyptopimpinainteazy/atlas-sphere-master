# Atlas Gateway

REST and GraphQL API gateway for querying Atlas Sphere indexed blockchain data.

## Features

- **REST API**: Full RESTful endpoints for blocks, extrinsics, events, Comits, and accounts
- **GraphQL**: Flexible GraphQL API with playground
- **PostgreSQL**: Queries indexed data from atlas-indexer database
- **CORS support**: Configurable cross-origin requests
- **Production-ready**: Graceful shutdown, health checks, tracing

## Prerequisites

- Rust 1.70+
- PostgreSQL 14+ with indexed data (from atlas-indexer)

## Quick Start

### 1. Configure

```bash
# Environment variables
export DATABASE_URL="postgres://gateway:gateway@localhost:5432/atlas_indexer"
export GATEWAY_HOST="127.0.0.1"
export GATEWAY_PORT="8080"

# Or use config file
cp gateway.toml gateway.local.toml
# Edit gateway.local.toml
```

### 2. Run

```bash
# Build
cargo build --release -p atlas-gateway

# Run
./target/release/atlas-gateway

# Or with config file
./target/release/atlas-gateway --config gateway.local.toml
```

## API Reference

### REST API v1

Base URL: `http://localhost:8080/api/v1`

#### Blocks

| Endpoint                         | Description                   |
| -------------------------------- | ----------------------------- |
| `GET /blocks`                    | Get recent blocks (paginated) |
| `GET /blocks/latest`             | Get latest indexed block      |
| `GET /blocks/:number`            | Get block by number           |
| `GET /blocks/:number/extrinsics` | Get block extrinsics          |
| `GET /blocks/:number/events`     | Get block events              |

#### Extrinsics

| Endpoint                | Description           |
| ----------------------- | --------------------- |
| `GET /extrinsics`       | Get recent extrinsics |
| `GET /extrinsics/:hash` | Get extrinsic by hash |

#### Events

| Endpoint                             | Description          |
| ------------------------------------ | -------------------- |
| `GET /events?pallet=...`             | Get events by pallet |
| `GET /events?pallet=...&variant=...` | Get events by type   |

#### Comits

| Endpoint            | Description                   |
| ------------------- | ----------------------------- |
| `GET /comits`       | Get recent Comit transactions |
| `GET /comits/:hash` | Get Comit by hash             |

#### Accounts

| Endpoint                            | Description            |
| ----------------------------------- | ---------------------- |
| `GET /accounts/:address`            | Get account details    |
| `GET /accounts/:address/extrinsics` | Get account extrinsics |
| `GET /accounts/:address/comits`     | Get account Comits     |

#### Statistics

| Endpoint     | Description          |
| ------------ | -------------------- |
| `GET /stats` | Get chain statistics |

### Pagination

Most list endpoints support pagination:

```
?limit=20&offset=0
```

Maximum limit is 100.

### GraphQL

Endpoint: `http://localhost:8080/graphql`

Playground: `http://localhost:8080/graphql/playground`

Example query:

```graphql
query {
  latestBlock {
    number
    hash
    timestamp
    extrinsicCount
    eventCount
  }
  
  stats {
    totalBlocks
    totalComits
    successfulComits
    totalAccounts
  }
  
  recentComits: comits(limit: 5) {
    comitHash
    origin
    success
    evmPayloadSize
    svmPayloadSize
  }
}
```

## Configuration

### Config File (gateway.toml)

```toml
[server]
host = "127.0.0.1"
port = 8080

[database]
url = "postgres://gateway:gateway@localhost:5432/atlas_indexer"
max_connections = 10
min_connections = 1

[cors]
allowed_origins = ["*"]
allowed_methods = ["GET", "POST", "OPTIONS"]
```

### Environment Variables

| Variable       | Description               | Default   |
| -------------- | ------------------------- | --------- |
| `DATABASE_URL` | PostgreSQL connection URL | Required  |
| `GATEWAY_HOST` | Server host               | 127.0.0.1 |
| `GATEWAY_PORT` | Server port               | 8080      |
| `RUST_LOG`     | Log level                 | info      |

## Development

```bash
# Run tests
cargo test -p atlas-gateway

# Check compilation
cargo check -p atlas-gateway

# Run with debug logging
RUST_LOG=atlas_gateway=debug cargo run -p atlas-gateway
```

## Architecture

```
┌─────────────────┐     ┌─────────────────┐
│  PostgreSQL     │────▶│  Atlas Gateway  │
│  (indexed data) │     │  (sqlx)         │
└─────────────────┘     └───────┬─────────┘
                                │
                    ┌───────────┼───────────┐
                    ▼           ▼           ▼
               /api/v1      /graphql    /health
               REST API     GraphQL     Status
```

## License

MIT OR Apache-2.0
