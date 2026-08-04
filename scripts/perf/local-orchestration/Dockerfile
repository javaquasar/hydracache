FROM ubuntu@sha256:561618e2c15bf2397621dd04f96926663a3b5616c189cf7e38db7e82f5c538ea

ARG ACTIONLINT_VERSION=1.7.7
ARG ACTIONLINT_SHA256=023070a287cd8cccd71515fedc843f1985bf96c436b7effaecce67290e7e0757

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
      bash ca-certificates coreutils curl findutils gawk git iproute2 jq \
      procps python3 shellcheck sudo systemd systemd-sysv util-linux \
    && rm -rf /var/lib/apt/lists/* \
    && curl --fail --location --silent --show-error \
      "https://github.com/rhysd/actionlint/releases/download/v${ACTIONLINT_VERSION}/actionlint_${ACTIONLINT_VERSION}_linux_amd64.tar.gz" \
      --output /tmp/actionlint.tar.gz \
    && echo "${ACTIONLINT_SHA256}  /tmp/actionlint.tar.gz" | sha256sum --check --strict \
    && tar --extract --gzip --file /tmp/actionlint.tar.gz --directory /usr/local/bin actionlint \
    && chmod 0755 /usr/local/bin/actionlint \
    && rm --force /tmp/actionlint.tar.gz

STOPSIGNAL SIGRTMIN+3
CMD ["/sbin/init"]
