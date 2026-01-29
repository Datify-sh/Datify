# Stage 1: Build Frontend
FROM oven/bun:1 AS frontend-builder

WORKDIR /app

COPY frontend/package.json ./
RUN bun install

COPY frontend/ ./
RUN bun run build

# Stage 2: Build Backend
FROM rust:1.88-bookworm AS backend-builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY backend/Cargo.toml backend/Cargo.lock ./

RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

COPY backend/src ./src
COPY backend/migrations ./migrations

RUN touch src/main.rs && cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy backend binary
COPY --from=backend-builder /app/target/release/datify /app/datify

# Copy backend migrations
COPY --from=backend-builder /app/migrations /app/migrations

# Copy frontend build
COPY --from=frontend-builder /app/dist /app/public

RUN mkdir -p /data

ENV DATABASE_URL=sqlite:/data/datify.db?mode=rwc
ENV LOG_LEVEL=info
ENV RUST_LOG=info

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["/app/datify"]
