#!/bin/bash
set -euo pipefail
cd /tmp/hc074-custody-loss-matrix-20261018.6G59my
source ./guard.sh
sha256sum ./tests-194 ./tests-188 ./guard.sh ./run.sh ./original_custody_owner_loss.py ./test_original_custody_owner_loss.py
for item in /proc/[0-9]*/exe; do
    executable=$(readlink "$item" 2>/dev/null || true)
    case "$executable" in
      /tmp/hc074-custody-loss-matrix-20261018.cQ3dLH/tests-*|/tmp/hc074-custody-loss-matrix-20261018.dJZndg/tests-*|/tmp/hc074-custody-loss-matrix-20261018.6G59my/tests-*)
        printf 'unexpected_fixture_executable=%s\n' "$executable"; exit 91 ;;
    esac
done
printf 'HC074-CUSTODY-LOSS-POSTFLIGHT unchanged=true negative_and_positive_absent=true admission=false\n'
