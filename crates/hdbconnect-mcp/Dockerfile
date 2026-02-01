# Build stage
FROM rust:1.88-bookworm AS builder
WORKDIR /build

# Cache dependencies
COPY Cargo.toml Cargo.lock ./
COPY crates/hdbconnect-mcp/Cargo.toml crates/hdbconnect-mcp/
COPY crates/hdbconnect-arrow/Cargo.toml crates/hdbconnect-arrow/
RUN mkdir -p crates/hdbconnect-mcp/src crates/hdbconnect-arrow/src && \
    echo "fn main() {}" > crates/hdbconnect-mcp/src/main.rs && \
    touch crates/hdbconnect-arrow/src/lib.rs && \
    cargo build --release -p hdbconnect-mcp && \
    rm -rf crates/*/src

# Build actual binary
COPY crates/ crates/
RUN cargo build --release -p hdbconnect-mcp --features http,telemetry

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/hdbconnect-mcp /usr/local/bin/

# Create entrypoint script with security warnings
RUN printf '#!/bin/sh\n\
if [ "$MCP_HTTP_HOST" = "0.0.0.0" ] || [ "$MCP_HTTP_HOST" = "::" ]; then\n\
  echo "WARNING: Binding to all interfaces ($MCP_HTTP_HOST). Ensure proper network security." >&2\n\
  if [ -z "$MCP_HTTP_BEARER_TOKEN" ]; then\n\
    echo "SECURITY WARNING: No authentication configured (MCP_HTTP_BEARER_TOKEN not set)." >&2\n\
    echo "Set MCP_HTTP_BEARER_TOKEN environment variable for production use." >&2\n\
  fi\n\
fi\n\
exec hdbconnect-mcp "$@"\n' > /entrypoint.sh && chmod +x /entrypoint.sh

# Non-root user
RUN useradd -r -u 1000 mcp
USER mcp

# Default to localhost for security; use 0.0.0.0 only with proper authentication
ENV MCP_TRANSPORT=http
ENV MCP_HTTP_HOST=127.0.0.1
ENV MCP_HTTP_PORT=8080
ENV RUST_LOG=info

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/entrypoint.sh"]
