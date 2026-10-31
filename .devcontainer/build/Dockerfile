FROM mcr.microsoft.com/devcontainers/rust:1

LABEL org.opencontainers.image.source="https://github.com/mellowagain/gitarena"

RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev cmake \
 && rm -rf /var/lib/apt/lists/*

RUN CARGO_HOME=/tmp/cargo cargo install --locked --root /usr/local \
    sqlx-cli --no-default-features --features postgres,rustls \
 && rm -rf /tmp/cargo

RUN curl -fsSL https://github.com/openai/codex/releases/latest/download/codex-x86_64-unknown-linux-musl.tar.gz \
    | tar -xz -C /tmp \
 && install -m 0755 /tmp/codex-x86_64-unknown-linux-musl /usr/local/bin/codex \
 && rm /tmp/codex-x86_64-unknown-linux-musl
