# Build stage
FROM rust:1.80 as builder
WORKDIR /app
ADD . /app
RUN cargo build --release

# Prod stage
FROM gcr.io/distroless/cc
COPY --from=builder /app/target/release/container-cubby /

EXPOSE 8602

CMD ["./container-cubby"]
