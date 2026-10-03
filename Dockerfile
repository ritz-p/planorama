ARG RUST_VERSION=1.85.1
FROM rust:${RUST_VERSION}-bookworm
RUN rustup component add rustfmt clippy
WORKDIR /workspace
ENV CARGO_TARGET_DIR=/workspace/target
CMD ["cargo", "run", "--", "--help"]
