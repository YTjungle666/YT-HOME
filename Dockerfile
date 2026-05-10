FROM node:24-alpine AS front-builder
ENV NPM_CONFIG_AUDIT=false \
    NPM_CONFIG_FUND=false \
    NPM_CONFIG_UPDATE_NOTIFIER=false
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci --ignore-scripts
COPY frontend/ ./
RUN npm run build

FROM rust:1.88.0-bookworm AS backend-builder
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock rustfmt.toml ./
COPY .cargo/ ./.cargo/
COPY packaging/ ./packaging/
COPY patches/ ./patches/
COPY vendor/ ./vendor/
COPY crates/ ./crates/
RUN if [ -x /app/packaging/docker/YTHOME ]; then \
      cp /app/packaging/docker/YTHOME /app/YTHOME; \
    else \
      rustup target add x86_64-unknown-linux-musl; \
      cargo build --offline --release --target x86_64-unknown-linux-musl -p app; \
      cp /app/target/x86_64-unknown-linux-musl/release/app /app/YTHOME; \
    fi

FROM golang:1.24-bookworm AS singbox-builder
ARG SING_BOX_VERSION=1.13.11
ARG SING_BOX_LINUX_LIBC=purego
WORKDIR /build
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git bash curl file python3 python3-requests unzip xz-utils gnupg \
    && rm -rf /var/lib/apt/lists/*
COPY scripts/build-sing-box.sh /usr/local/bin/build-sing-box
COPY packaging/ ./packaging/
RUN if [ -x /build/packaging/docker/sing-box ]; then \
      mkdir -p /opt/sing-box; \
      cp /build/packaging/docker/sing-box /opt/sing-box/sing-box; \
    else \
      SING_BOX_LINUX_LIBC="${SING_BOX_LINUX_LIBC}" sh /usr/local/bin/build-sing-box linux amd64 /opt/sing-box "${SING_BOX_VERSION}"; \
    fi; \
    /opt/sing-box/sing-box version | grep -F -q "${SING_BOX_VERSION}"; \
    /opt/sing-box/sing-box version | grep -F -q with_v2ray_api

FROM alpine:3.22
LABEL org.opencontainers.image.title="YT-HOME"
LABEL org.opencontainers.image.description="Rust control plane for sing-box based home access."
LABEL org.opencontainers.image.source="https://github.com/YTjungle666/YT-HOME"
LABEL org.opencontainers.image.licenses="GPL-3.0-only"
ENV YTHOME_WEB_DIR=/app/web
ENV YTHOME_MIGRATIONS_DIR=/app/migrations
WORKDIR /app
RUN apk add --no-cache ca-certificates tzdata gcompat openrc openssh-server openssh-keygen \
    && mkdir -p /app/db
COPY --chmod=755 scripts/container-init.sh /usr/local/bin/container-init
COPY --chmod=755 --from=backend-builder /app/YTHOME /app/YTHOME
COPY --chmod=755 --from=singbox-builder /opt/sing-box/ /app/
COPY --from=front-builder /app/frontend/dist/ /app/web/
COPY crates/infra-db/migrations/ /app/migrations/
COPY --chmod=755 packaging/openrc/YTHOME /etc/init.d/YT-HOME
RUN rc-update add YT-HOME default \
    && rm -f /sbin/init \
    && cp /usr/local/bin/container-init /sbin/init
EXPOSE 80 2096 22
CMD ["/usr/local/bin/container-init"]
