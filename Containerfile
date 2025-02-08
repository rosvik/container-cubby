# Setup cargo-chef
FROM rust:1.81-alpine3.20 as chef
RUN apk add --no-cache musl-dev
RUN cargo install cargo-chef
WORKDIR /app

# Chef prepare: Analyze the current project to determine the
# minimum subset of files (Cargo.lock and Cargo.toml
# manifests) required to build it and cache dependencies
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Build stage
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# see https://github.com/rust-lang/docker-rust/issues/85
ENV RUSTFLAGS="-C target-feature=-crt-static"
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY ./ /app
RUN cargo build --release
RUN strip target/release/container-cubby

# Prod stage (alpine version must match build stage)
FROM alpine:3.20
RUN apk add --no-cache libgcc
COPY --from=builder /app/target/release/container-cubby /
ENTRYPOINT ["/container-cubby"]
