FROM rust:1.74-slim AS builder

WORKDIR /app

# Install required system dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml ./
COPY build.rs ./
COPY src ./src
COPY VERSION ./

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/odd-dashboard /usr/local/bin/

ENV READ_MODEL_URL=http://read-model:8080

CMD ["odd-dashboard"]
