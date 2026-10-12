# syntax=docker/dockerfile:1.7

FROM debian:bookworm-slim AS runtime-base

ENV FILFINANCE_PIPER_VENV=/opt/filfinance/piper-venv
ENV FILFINANCE_OPUS_MT_VENV=/opt/filfinance/opus-mt-venv
ENV FILFINANCE_OPUS_MT_MODEL_DIR=/opt/filfinance/models/opus-mt-tc-big-en-fr-ct2-int8
ENV PIP_DISABLE_PIP_VERSION_CHECK=1

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bash \
        ca-certificates \
        curl \
        espeak-ng \
        espeak-ng-data \
        ffmpeg \
        libespeak-ng1 \
        libgomp1 \
        libsndfile1 \
        libssl3 \
        python3 \
        python3-venv \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
RUN mkdir -p /app/scripts
COPY scripts/requirements-piper.txt /app/scripts/requirements-piper.txt
COPY scripts/requirements-opus-mt.txt /app/scripts/requirements-opus-mt.txt
COPY scripts/install_opus_mt_model.py /app/scripts/install_opus_mt_model.py
RUN --mount=type=cache,id=filfinance-piper-pip-cache,target=/root/.cache/pip \
    python3 -m venv "$FILFINANCE_PIPER_VENV" \
    && "$FILFINANCE_PIPER_VENV/bin/python" -m pip install --upgrade pip \
    && "$FILFINANCE_PIPER_VENV/bin/python" -m pip install -r /app/scripts/requirements-piper.txt
RUN --mount=type=cache,id=filfinance-opus-mt-pip-cache,target=/root/.cache/pip \
    python3 -m venv "$FILFINANCE_OPUS_MT_VENV" \
    && "$FILFINANCE_OPUS_MT_VENV/bin/python" -m pip install --upgrade pip \
    && "$FILFINANCE_OPUS_MT_VENV/bin/python" -m pip install -r /app/scripts/requirements-opus-mt.txt
RUN --mount=type=cache,id=filfinance-huggingface-cache,target=/root/.cache/huggingface \
    "$FILFINANCE_OPUS_MT_VENV/bin/python" /app/scripts/install_opus_mt_model.py \
    --output-dir "$FILFINANCE_OPUS_MT_MODEL_DIR"

FROM rust:1.96.0-slim-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libssl-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

# L'image Rust contient deja cargo/rustc pour ce toolchain, mais pas forcement
# les composants de developpement declares dans rust-toolchain.toml. En build
# Docker, on force le toolchain installe pour eviter une synchro rustup reseau.
ENV RUSTUP_TOOLCHAIN=1.96.0

WORKDIR /app
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
COPY vendor ./vendor
RUN --mount=type=cache,id=filfinance-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=filfinance-cargo-git,target=/usr/local/cargo/git \
    --mount=type=cache,id=filfinance-service-target,target=/app/target \
    cargo build --release \
    && install -Dm755 /app/target/release/filfinance-service /usr/local/bin/filfinance-service

FROM runtime-base

WORKDIR /app
COPY --from=builder /usr/local/bin/filfinance-service /usr/local/bin/filfinance-service
COPY .env.example /app/.env.example
COPY scripts ./scripts
RUN mkdir -p /app/data \
    && chmod +x /app/scripts/bootstrap_runtime_assets.sh /app/scripts/bootstrap_piper_local.sh

EXPOSE 8088
ENTRYPOINT ["/app/scripts/bootstrap_runtime_assets.sh"]
CMD ["filfinance-service"]
