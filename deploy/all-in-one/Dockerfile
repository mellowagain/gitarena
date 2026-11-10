# The backend, frontend, caddy and setup contexts only exist when built through docker-bake.hcl

ARG CADDY_VERSION=2.11.9

FROM caddy:${CADDY_VERSION}-alpine AS caddy-bin

FROM golang:trixie AS caddy-licenses

ARG CADDY_VERSION

RUN go install github.com/google/go-licenses/v2@v2.0.1

COPY <<'EOF' /go-licenses.tpl
Third-party licenses
====================

Caddy includes the following third-party software.
{{ range . }}
--------------------------------------------------------------------------------
{{ .Name }} {{ .Version }} ({{ .LicenseName }})

{{ .LicenseText }}
{{ end }}
EOF

WORKDIR /caddy-licenses
RUN go mod init licenses \
    && go get github.com/caddyserver/caddy/v2/cmd/caddy@v${CADDY_VERSION} \
    && go-licenses report --template /go-licenses.tpl github.com/caddyserver/caddy/v2/cmd/caddy > THIRD_PARTY_LICENSES.txt \
    && printf -- '--------------------------------------------------------------------------------\nGo standard library (BSD-3-Clause)\n\n' >> THIRD_PARTY_LICENSES.txt \
    && cat "$(go env GOROOT)/LICENSE" >> THIRD_PARTY_LICENSES.txt

FROM backend

SHELL ["/bin/bash", "-euo", "pipefail", "-c"]

ARG TARGETARCH

RUN install -d /usr/share/postgresql-common/pgdg \
    && curl -fsSLo /usr/share/postgresql-common/pgdg/apt.postgresql.org.asc https://www.postgresql.org/media/keys/ACCC4CF8.asc \
    && echo "deb [signed-by=/usr/share/postgresql-common/pgdg/apt.postgresql.org.asc] https://apt.postgresql.org/pub/repos/apt trixie-pgdg main" > /etc/apt/sources.list.d/pgdg.list \
    && apt-get update \
    && apt-get install -y postgresql-client-18 xz-utils \
    && rm -rf /var/lib/apt/lists/*

RUN version=3.2.3.2; \
    case "$TARGETARCH" in \
        amd64) arch=x86_64; sum=e6befcc96a437a3831386ecfc51808c5d3e939dc5fe3c02ae9284599e8aa2408 ;; \
        arm64) arch=aarch64; sum=b17f17a82e7a515c682a91edaf2ffdabb73f891981b6c1fd712115693a2f8b4c ;; \
    esac; \
    base="https://github.com/just-containers/s6-overlay/releases/download/v$version"; \
    curl -fsSLo /tmp/s6-noarch.tar.xz "$base/s6-overlay-noarch.tar.xz"; \
    curl -fsSLo /tmp/s6-arch.tar.xz "$base/s6-overlay-$arch.tar.xz"; \
    printf '%s  %s\n' \
        5379750ed30a84bbd2e2dd74847ba6b5bd29cd0b2e3ea2ec58049b57eb2eda12 /tmp/s6-noarch.tar.xz \
        "$sum" /tmp/s6-arch.tar.xz | sha256sum -c -; \
    tar -C / -Jxpf /tmp/s6-noarch.tar.xz; \
    tar -C / -Jxpf /tmp/s6-arch.tar.xz; \
    rm /tmp/s6-*.tar.xz

COPY --from=caddy-bin /usr/bin/caddy /usr/bin/caddy
COPY --from=caddy-licenses /caddy-licenses/THIRD_PARTY_LICENSES.txt /usr/share/licenses/caddy/THIRD_PARTY_LICENSES.txt
COPY --from=frontend /usr/local/bin/node /usr/local/bin/node
COPY --from=frontend /usr/local/LICENSE /usr/share/licenses/node/LICENSE
COPY --from=frontend /app /frontend
COPY --from=caddy Caddyfile /etc/caddy/Caddyfile
COPY --from=setup 10-base.sql /setup/10-base.sql
COPY rootfs/ /

ENV BIND_ADDRESS="127.0.0.1:8080" \
    GITARENA_BACKEND="127.0.0.1:8080" \
    GITARENA_FRONTEND="127.0.0.1:3000" \
    GITARENA_SCHEME="http" \
    GITARENA_DOMAIN="localhost" \
    XDG_DATA_HOME="/data" \
    XDG_CONFIG_HOME="/config" \
    S6_BEHAVIOUR_IF_STAGE2_FAILS=2 \
    S6_CMD_WAIT_FOR_SERVICES_MAXTIME=0

EXPOSE 80 443 443/udp 2222

HEALTHCHECK CMD curl -fsS -o /dev/null http://127.0.0.1:8080/healthz && curl -fsS -o /dev/null http://127.0.0.1:3000/

ENTRYPOINT ["/init"]
CMD []
