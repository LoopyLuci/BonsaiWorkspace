# Multi-stage Dockerfile for Omnisystem

# Stage 1: Builder
FROM rust:latest as builder

WORKDIR /app

# Copy project files
COPY . .

# Build Omnisystem
WORKDIR /app/Omnisystem
RUN cargo build --release

# Stage 2: Runtime
FROM ubuntu:22.04

WORKDIR /omnisystem

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy built binary from builder
COPY --from=builder /app/Omnisystem/target/release/omnisystem* /usr/local/bin/

# Copy documentation
COPY --from=builder /app/Omnisystem/docs /omnisystem/docs

# Create app directory
RUN mkdir -p /omnisystem/config /omnisystem/data /omnisystem/logs

# Set permissions
RUN chmod +x /usr/local/bin/omnisystem* || true

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD omnisystem --health || exit 1

# Expose ports
EXPOSE 8080 9090 5432

# Entry point
ENTRYPOINT ["omnisystem"]
CMD ["--help"]

# Labels
LABEL maintainer="rechargedideas@gmail.com"
LABEL description="Omnisystem - Complete Operating System for 100 Years"
LABEL version="3.0.0"
