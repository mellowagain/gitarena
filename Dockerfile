FROM rust:trixie AS chef

RUN cargo install cargo-chef --locked
WORKDIR /usr/src/gitarena

FROM chef AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /usr/src/gitarena/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .

RUN cargo build --release

FROM golang:trixie AS zoekt-builder

RUN go install github.com/sourcegraph/zoekt/cmd/zoekt-git-index@latest

FROM debian:trixie-slim

RUN apt-get update && apt-get install -y git curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/gitarena/target/release/gitarena /app/gitarena-bin
COPY --from=zoekt-builder /go/bin/zoekt-git-index /usr/local/bin/zoekt-git-index

EXPOSE 8080
EXPOSE 2222

ENV BIND_ADDRESS="0.0.0.0:8080"
ENTRYPOINT ["/app/gitarena-bin"]
