# syntax=docker/dockerfile:1

# Base images are pinned by digest so a published tag is reproducible from its
# source commit. Bump the tag and the digest together; the builder toolchain
# tracks the toolchain CI uses (dtolnay/rust-toolchain in ci.yml).
ARG RUST_IMAGE=rust:1.94.0-bookworm@sha256:365468470075493dc4583f47387001854321c5a8583ea9604b297e67f01c5a4f
ARG RUNTIME_IMAGE=gcr.io/distroless/cc-debian12:nonroot@sha256:9dac0a79194e45a7da0158a9c6da57b217585af0786db3845d1f0ec1a0dd182f

FROM ${RUST_IMAGE} AS builder
WORKDIR /workspace
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/workspace/target \
    cargo build --release --locked -p hydracache-server \
    && mkdir -p /out/bin /out/data \
    && cp target/release/hydracache-server /out/bin/hydracache-server

FROM ${RUNTIME_IMAGE}
ARG VERSION=dev
ARG REVISION=unknown
LABEL org.opencontainers.image.title="HydraCache Server" \
      org.opencontainers.image.description="Standalone HydraCache production daemon" \
      org.opencontainers.image.source="https://github.com/javaquasar/hydracache" \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.revision="${REVISION}"
COPY --from=builder /out/bin/hydracache-server /usr/local/bin/hydracache-server
# distroless has no shell, so the storage directory is created in the builder
# and handed to the nonroot (65532) user here.
COPY --from=builder --chown=65532:65532 /out/data /var/lib/hydracache
ENV HYDRACACHE_ROLE=member
ENV HYDRACACHE_LISTEN_ADDR=0.0.0.0:8080
ENV HYDRACACHE_CLUSTER_ADDR=0.0.0.0:7000
ENV HYDRACACHE_ADMIN_ADDR=0.0.0.0:9091
ENV HYDRACACHE_STORAGE_DIR=/var/lib/hydracache
# 8080 client HTTP, 7000 cluster, 9091 admin (/healthz, /readyz).
EXPOSE 8080 7000 9091
USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/hydracache-server"]
