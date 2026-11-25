# Stage 1: Build React Frontend
FROM node:22-alpine AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci || npm install
COPY frontend/ ./
RUN npm run build

# Stage 2: Build Rust Server
FROM rust:1-bookworm AS backend-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY crates/ ./crates/
RUN cargo build --release -p traveller-server

# Stage 3: Minimal Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend-builder /app/target/release/traveller-server /usr/local/bin/traveller-server
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist
COPY packs/starter-srd /app/packs/starter-srd

ENV PORT=3080
ENV FRONTEND_DIR=/app/frontend/dist
ENV DATA_PACKS_DIR=/data/packs
EXPOSE 3080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -fsS http://localhost:${PORT}/api/health || exit 1

CMD ["traveller-server"]
