# Build multi-stage otimizado: cache de deps + runtime distroless
FROM rust:1.89-slim-bookworm AS builder
WORKDIR /app
# Cache de dependências: copia só manifests primeiro
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release --bin instrumented_pipeline && \
    cp target/release/instrumented_pipeline /tmp/app

FROM gcr.io/distroless/cc-debian12 AS runtime
COPY --from=builder /tmp/app /app
ENTRYPOINT ["/app"]
