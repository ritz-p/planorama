ARG RUST_VERSION=1.85.1
FROM rust:${RUST_VERSION}-bookworm
RUN rustup component add rustfmt clippy
RUN apt-get update && apt-get install -y --no-install-recommends librsvg2-bin fonts-dejavu-core \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /workspace
ENV CARGO_TARGET_DIR=/workspace/target
CMD ["cargo", "run", "--", "--help"]
