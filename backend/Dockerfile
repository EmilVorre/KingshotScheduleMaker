# Multi-stage Rust build - API only (no static files)
FROM rust:1-bookworm AS builder
WORKDIR /app

# Copy manifest first for layer caching
COPY prep-appointments/Cargo.toml prep-appointments/Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release && rm -rf src

# Copy source and rebuild
COPY prep-appointments/src ./src
RUN touch src/main.rs && cargo build --release

# Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/prep-appointments .
RUN mkdir -p data

ENV PORT=8080
EXPOSE 8080

CMD ["sh", "-c", "exec ./prep-appointments web ${PORT}"]
