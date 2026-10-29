#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

repo_root="$(git rev-parse --show-toplevel)"
exec python3 "$repo_root/scripts/perf/reference_campaign.py" "$@"
