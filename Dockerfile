ARG RUST_VERSION=1.85.1
FROM rust:${RUST_VERSION}-bookworm
COPY --from=hashicorp/terraform:1.9.8 /bin/terraform /usr/local/bin/terraform
RUN rustup component add rustfmt clippy
RUN apt-get update && apt-get install -y --no-install-recommends librsvg2-bin fonts-dejavu-core chromium chromium-driver nodejs jq time \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /workspace
ENV CARGO_TARGET_DIR=/workspace/target
CMD ["cargo", "run", "--", "--help"]
