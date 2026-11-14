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

RUN cargo build --locked --release
RUN cargo about generate about.hbs -o THIRD_PARTY_LICENSES.txt

FROM golang:trixie AS zoekt-builder

ARG ZOEKT_VERSION=v0.0.0-20261103080450-a5a933bc043e

RUN go install github.com/sourcegraph/zoekt/cmd/zoekt-git-index@${ZOEKT_VERSION} \
    && go install github.com/sourcegraph/zoekt/cmd/zoekt-webserver@${ZOEKT_VERSION}

RUN go install github.com/google/go-licenses/v2@v2.0.1

COPY <<'EOF' /go-licenses.tpl
Third-party licenses
====================

zoekt includes the following third-party software.
{{ range . }}
--------------------------------------------------------------------------------
{{ .Name }} {{ .Version }} ({{ .LicenseName }})

{{ .LicenseText }}
{{ end }}
EOF

WORKDIR /zoekt-licenses
RUN go mod init licenses \
    && go get github.com/sourcegraph/zoekt/cmd/zoekt-git-index@${ZOEKT_VERSION} \
        github.com/sourcegraph/zoekt/cmd/zoekt-webserver@${ZOEKT_VERSION} \
    && go-licenses report --template /go-licenses.tpl \
        github.com/sourcegraph/zoekt/cmd/zoekt-git-index \
        github.com/sourcegraph/zoekt/cmd/zoekt-webserver > THIRD_PARTY_LICENSES.txt \
    && printf -- '--------------------------------------------------------------------------------\nGo standard library (BSD-3-Clause)\n\n' >> THIRD_PARTY_LICENSES.txt \
    && cat "$(go env GOROOT)/LICENSE" >> THIRD_PARTY_LICENSES.txt

FROM debian:trixie-slim AS zoekt-webserver

COPY --from=zoekt-builder /go/bin/zoekt-webserver /usr/local/bin/zoekt-webserver
COPY --from=zoekt-builder /zoekt-licenses/THIRD_PARTY_LICENSES.txt /usr/share/licenses/zoekt/THIRD_PARTY_LICENSES.txt

EXPOSE 6070
ENTRYPOINT ["/usr/local/bin/zoekt-webserver"]

FROM debian:trixie-slim

RUN apt-get update && apt-get install -y git curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/gitarena/target/release/gitarena /app/gitarena-bin
COPY --from=builder /usr/src/gitarena/THIRD_PARTY_LICENSES.txt /app/THIRD_PARTY_LICENSES.txt
COPY LICENSE.txt /app/LICENSE.txt
COPY --from=zoekt-builder /go/bin/zoekt-git-index /usr/local/bin/zoekt-git-index
COPY --from=zoekt-builder /zoekt-licenses/THIRD_PARTY_LICENSES.txt /usr/share/licenses/zoekt/THIRD_PARTY_LICENSES.txt

EXPOSE 8080
EXPOSE 2222

ENV BIND_ADDRESS="0.0.0.0:8080"
ENTRYPOINT ["/app/gitarena-bin"]
