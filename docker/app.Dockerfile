# syntax=docker/dockerfile:1.7
# finbot image: cargo-chef caches dependency builds; the final stage is
# distroless (glibc, CA certificates, no shell) running as non-root.

ARG RUST_VERSION=1.95

FROM rust:${RUST_VERSION}-slim-trixie AS chef
RUN cargo install cargo-chef --locked --version 0.1.78
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json --package finbot
COPY . .
# Query macros read the committed .sqlx/ data; no database at build time.
ENV SQLX_OFFLINE=true
RUN cargo build --release --locked --package finbot \
    && cp target/release/finbot /usr/local/bin/finbot

# Debian 13 matches the builder's glibc.
FROM gcr.io/distroless/cc-debian13:nonroot
COPY --from=builder /usr/local/bin/finbot /usr/local/bin/finbot
ENV HTTP_BIND=0.0.0.0:8080 \
    LOG_FORMAT=json
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/finbot"]
CMD ["serve"]
