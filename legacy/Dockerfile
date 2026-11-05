# syntax=docker/dockerfile:1

# ---- build stage ----------------------------------------------------------
FROM rust:1-slim-bookworm AS builder

# native-tls (via reqwest + tokio-tungstenite) needs OpenSSL headers at build time
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Build dependencies first so source changes don't recompile the world.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && cargo build --release --locked \
    && rm -rf src

# Now the real source.
COPY src ./src
RUN touch src/main.rs && cargo build --release --locked

# ---- runtime stage --------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 aloha

COPY --from=builder /app/target/release/aloha /usr/local/bin/aloha

USER aloha
ENV RUST_LOG=info

# Token is supplied at runtime: docker run -e FLUXER_TOKEN=...
ENTRYPOINT ["/usr/local/bin/aloha"]
