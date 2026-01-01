# ============================================================================
# X3 ATLAS SPHERE - SWARM MEDIA SERVICE DOCKERFILE
# Production-grade container for autonomous marketing swarm
# ============================================================================

FROM rust:1.75 as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release --package swarm-media

# ============================================================================
# Runtime stage
# ============================================================================

FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libsqlite3-0 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/swarm-media /app/swarm-media

# Create non-root user
RUN useradd -m -u 1000 swarm && chown -R swarm:swarm /app
USER swarm

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Expose ports
EXPOSE 8080 8081 8082

# Environment variables
ENV RUST_LOG=info
ENV SWARM_ENV=production

# Run application
CMD ["./swarm-media"]
