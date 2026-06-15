# Phase 10: Production Deployment - Omnisystem Docker Image

FROM rust:1.75 as builder

WORKDIR /build

# Copy workspace
COPY . .

# Build release binary
RUN cargo build --release --bin omnisystem-bin 2>&1 | head -20

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/omnisystem-bin .

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:9090/health || exit 1

# Expose ports
EXPOSE 8080 8081 9090

# Run
CMD ["./omnisystem-bin"]
