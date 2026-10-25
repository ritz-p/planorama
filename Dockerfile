ARG RUST_VERSION=1.85.1
FROM rust:${RUST_VERSION}-bookworm
COPY --from=hashicorp/terraform:1.9.8 /bin/terraform /usr/local/bin/terraform
RUN rustup component add rustfmt clippy
RUN rustup target add wasm32-unknown-unknown \
    && cargo install wasm-bindgen-cli --version 0.2.100 --locked
RUN apt-get update && apt-get install -y --no-install-recommends librsvg2-bin fonts-dejavu-core chromium chromium-driver nodejs jq time curl unzip \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /workspace
COPY scripts/install-mold.sh scripts/native-linker.sh /tmp/planorama-tools/
RUN sh /tmp/planorama-tools/install-mold.sh \
    && install -m 755 /tmp/planorama-tools/native-linker.sh /usr/local/bin/planorama-native-linker \
    && rm -rf /tmp/planorama-tools
ENV CARGO_TARGET_DIR=/workspace/target
CMD ["cargo", "run", "--", "--help"]
