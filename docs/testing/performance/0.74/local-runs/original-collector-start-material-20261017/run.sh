#!/bin/sh
set -eu
umask 077
cd /tmp/hc074-start-material-20261017.vLX1GL
. ./guard.sh
lane="$1"
case "$lane" in 194|188) ;; *) exit 81 ;; esac
case "$lane" in 194) expected=674196c75a848ae84bc4af6d80866a918d10dc92d9d3c9e5169975247522f60e ;; 188) expected=c3658924f11734c0552f12eba6c911bd5217bdd88117122926917718b6ac0f45 ;; esac
[ "$(sha256sum "./tests-$lane" | cut -d' ' -f1)" = "$expected" ]
[ ! -e "./run-$lane" ]
mkdir -m 700 "./run-$lane"
export TMPDIR="$PWD/run-$lane"
export HC074_START_MATERIAL_BUNDLE="$PWD/bundle"
sha256sum "$HC074_START_MATERIAL_BUNDLE/"*
timeout 120s "./tests-$lane" 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::original_start_material_joined_healthy' --ignored --exact --nocapture --test-threads=1 > "./healthy-$lane.log" 2>&1
cat "./healthy-$lane.log"
preflight
timeout 360s "./tests-$lane" 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::original_start_material_joined_refusals' --ignored --exact --nocapture --test-threads=1 > "./refusals-$lane.log" 2>&1
cat "./refusals-$lane.log"
preflight
# End.
