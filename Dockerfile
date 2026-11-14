FROM rust:1-slim-bookworm AS builder
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config cmake perl make gcc g++ \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main(){}" > src/main.rs && cargo build --release && rm -rf src
COPY src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates curl procps \
      chromium xvfb x11vnc openbox novnc websockify \
      fonts-liberation fonts-noto-color-emoji fonts-noto-cjk \
 && rm -rf /var/lib/apt/lists/* \
 && useradd -r -u 10001 -m -d /data app \
 && mkdir -p /data /tmp/.X11-unix /app \
 && chmod 1777 /tmp/.X11-unix \
 && cp -r /usr/share/novnc /app/novnc \
 && printf '<!doctype html><meta charset="utf-8"><title>Sina Browser</title><meta http-equiv="refresh" content="0;url=vnc.html?autoconnect=true&resize=scale&reconnect=true&path=websockify">' > /app/novnc/index.html \
 && chown -R app /data
COPY --from=builder /app/target/release/sina /usr/local/bin/sina
COPY static /app/static
WORKDIR /app
ENV PORT=8080 VIEW_PORT=8081 STATIC_DIR=/app/static NOVNC_DIR=/app/novnc DATA_DIR=/data
VOLUME /data
EXPOSE 8080 8081
USER app
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s \
  CMD curl -fsS http://localhost:8080/healthz || exit 1
CMD ["sina"]
