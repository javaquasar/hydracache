#!/usr/bin/env bash
set -euo pipefail

# A protected self-hosted measurement machine only checks its admitted packages.
# Hosted CI installs the build headers and isolated test bus before compiling tools.
if [[ ${HYDRACACHE_RUNNER_ENVIRONMENT:-self-hosted} == github-hosted ]]; then
  sudo -n apt-get update -qq
  sudo -n apt-get install --yes --no-install-recommends libdbus-1-dev pkg-config dbus-daemon
fi

pkg-config --modversion dbus-1
command -v dbus-daemon
