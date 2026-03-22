# ObjStor Dockerfile
# Multi-stage build for minimal image size

# Stage 1: Build
FROM rust:1.70-alpine AS builder

# Install build dependencies
RUN apk add --no-cache \
    musl-dev \
    sqlite-dev \
    pkgconfig \
    openssl-dev

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Build the application
RUN cargo build --release

# Stage 2: Runtime
FROM alpine:3.18

# Install runtime dependencies
RUN apk add --no-cache \
    sqlite-libs \
    ca-certificates

# Create non-root user
RUN addgroup -g 1000 objstor && \
    adduser -D -u 1000 -G objstor objstor

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/objstor /app/objstor

# Create necessary directories
RUN mkdir -p /app/data/config /app/data/pools /app/logs && \
    chown -R objstor:objstor /app

# Switch to non-root user
USER objstor

# Expose ports
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV OBJSTOR_DATA_DIR=/app/data

# Run the application
CMD ["/app/objstor"]
