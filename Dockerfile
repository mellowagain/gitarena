FROM rust AS builder

WORKDIR /usr/src/gitarena
COPY . .

RUN cargo build --release

FROM debian:buster-slim
RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/gitarena/target/x86_64-unknown-linux-gnu/release/gitarena /app/

EXPOSE 8080
ENV BIND_ADDRESS="localhost:8080"
ENTRYPOINT ["/app/gitarena"]
