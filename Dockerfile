FROM rust:1.85-bookworm
RUN rustup component add rustfmt clippy
WORKDIR /workspace
ENV CARGO_TARGET_DIR=/workspace/target
CMD ["cargo", "run", "--", "--help"]
