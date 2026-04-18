FROM rust:trixie AS builder

WORKDIR /usr/src/gitarena
COPY . .

RUN cargo build --release

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/gitarena/target/release/gitarena /app/

EXPOSE 8080
ENV BIND_ADDRESS="0.0.0.0:8080"
ENTRYPOINT ["/app/gitarena"]
