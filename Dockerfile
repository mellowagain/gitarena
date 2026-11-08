FROM rust:trixie AS chef

RUN cargo install cargo-chef --locked
RUN cargo install cargo-about --locked --features cli
WORKDIR /usr/src/gitarena

FROM chef AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY --from=planner /usr/src/gitarena/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .

ARG VERGEN_GIT_SHA=unknown

RUN cargo build --release
RUN cargo about generate about.hbs -o THIRD_PARTY_LICENSES.txt

FROM golang:trixie AS zoekt-builder

ARG ZOEKT_VERSION=v0.0.0-20261103080450-a5a933bc043e

RUN go install github.com/sourcegraph/zoekt/cmd/zoekt-git-index@${ZOEKT_VERSION} \
    && go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@${ZOEKT_VERSION}

# Built from the same stage as the backend's zoekt-git-index so the index format always matches
FROM debian:trixie-slim AS zoekt-webserver

COPY --from=zoekt-builder /go/bin/zoekt-webserver /usr/local/bin/zoekt-webserver

EXPOSE 6070
ENTRYPOINT ["/usr/local/bin/zoekt-webserver"]

FROM debian:trixie-slim

RUN apt-get update && apt-get install -y git curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/gitarena/target/release/gitarena /app/gitarena-bin
COPY --from=builder /usr/src/gitarena/THIRD_PARTY_LICENSES.txt /app/THIRD_PARTY_LICENSES.txt
COPY --from=zoekt-builder /go/bin/zoekt-git-index /usr/local/bin/zoekt-git-index

EXPOSE 8080
EXPOSE 2222

ENV BIND_ADDRESS="0.0.0.0:8080"
ENTRYPOINT ["/app/gitarena-bin"]
