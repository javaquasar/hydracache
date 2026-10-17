#!/bin/sh
set -eu
umask 077
cd /tmp/hc074-start-material-20261017.vLX1GL
. ./guard.sh
lane="$1"
case "$lane" in 194|188) ;; *) exit 81 ;; esac
case "$lane" in 194) expected=62b8827ea81ae7aff6f739a330103a4dfb688fe4cd4679f3f311968b7218647e ;; 188) expected=427cb197d105db8106c1394b5686f9bb863f6b604a8d6e77f575c933a45086bd ;; esac
[ "$(sha256sum "./tests-v2-$lane" | cut -d' ' -f1)" = "$expected" ]
[ ! -e "./run-v2-$lane" ]
mkdir -m 700 "./run-v2-$lane"
export TMPDIR="$PWD/run-v2-$lane"
export HC074_START_MATERIAL_BUNDLE="$PWD/bundle"
sha256sum "$HC074_START_MATERIAL_BUNDLE/"*
timeout 120s "./tests-v2-$lane" 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::original_start_material_joined_healthy' --ignored --exact --nocapture --test-threads=1 > "./healthy-v2-$lane.log" 2>&1
cat "./healthy-v2-$lane.log"
preflight
timeout 360s "./tests-v2-$lane" 'diagnostic_worker_policy::trusted_input::controller_input::build_input::oidc_input::tests::fence_preflight_tests::attempt_tests::collector_tests::observed_reservation_tests::collector_reserved_tests::progress_tests::start_consent_tests::validation_frame_tests::starting_tests::sink_tests::start_material_tests::original_start_material_joined_refusals' --ignored --exact --nocapture --test-threads=1 > "./refusals-v2-$lane.log" 2>&1
cat "./refusals-v2-$lane.log"
preflight
# End.
