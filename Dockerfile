# syntax=docker/dockerfile:1

FROM rust:1.88-bookworm AS builder
WORKDIR /workspace
COPY . .
RUN cargo build --release --locked -p hydracache-server

FROM gcr.io/distroless/cc-debian12:nonroot
LABEL org.opencontainers.image.title="HydraCache Server"
LABEL org.opencontainers.image.description="Standalone HydraCache production daemon"
COPY --from=builder /workspace/target/release/hydracache-server /usr/local/bin/hydracache-server
ENV HYDRACACHE_ROLE=member
ENV HYDRACACHE_LISTEN_ADDR=0.0.0.0:8080
ENV HYDRACACHE_CLUSTER_ADDR=0.0.0.0:7000
ENV HYDRACACHE_STORAGE_DIR=/var/lib/hydracache
USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/hydracache-server"]
