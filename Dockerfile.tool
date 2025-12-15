FROM rust:1.81 AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM gcr.io/distroless/static-debian12
COPY --from=builder /app/target/release/embeddenator /bin/embeddenator
ENTRYPOINT ["/bin/embeddenator"]
