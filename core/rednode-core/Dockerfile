FROM rust:1.82 as builder
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/rednode-core /usr/local/bin/rednode-core
EXPOSE 8787
CMD ["rednode-core"]
