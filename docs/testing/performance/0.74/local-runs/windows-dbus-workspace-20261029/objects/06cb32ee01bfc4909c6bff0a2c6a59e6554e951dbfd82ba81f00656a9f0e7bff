#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

repo_root="$(git rev-parse --show-toplevel)"
source_dir="/var/lib/hydracache-perf/reference-campaign-v1"
source_receipt="$source_dir/reference-campaign-admission.json"
source_bundle="$source_dir/reference-campaign-host-admission.tar.gz"
target_dir="$repo_root/target/test-evidence/0.67.1/reference-campaign"
target_receipt="$target_dir/reference-campaign-admission.json"
target_bundle="$target_dir/reference-campaign-host-admission.tar.gz"
profile="$repo_root/docs/testing/perf-host-profiles/ubuntu-24.04-reference-v1.json"

test "$(id --user --name)" = github-runner || {
  echo "reference campaign admission import must run as github-runner" >&2
  exit 1
}
for tool in git jq sha256sum stat; do
  command -v "$tool" >/dev/null || {
    echo "missing campaign admission import tool: $tool" >&2
    exit 1
  }
done
test "$(stat --format=%U "$source_dir")" = root
test "$(stat --format=%G "$source_dir")" = root
test "$(stat --format=%a "$source_dir")" = 555
for source in "$source_receipt" "$source_bundle"; do
  test -f "$source"
  test ! -L "$source"
  test "$(stat --format=%U "$source")" = root
  test "$(stat --format=%G "$source")" = root
  test "$(stat --format=%a "$source")" = 444
done

source_commit="$(git -C "$repo_root" rev-parse HEAD)"
profile_sha256="$(sha256sum "$profile" | awk '{print $1}')"
bundle_sha256="$(sha256sum "$source_bundle" | awk '{print $1}')"
jq --exit-status \
  --arg source_commit "$source_commit" \
  --arg profile_sha256 "$profile_sha256" \
  --arg bundle_sha256 "$bundle_sha256" '
    (keys | sort) == ([
      "bootstrap_evidence",
      "campaign_id",
      "host_admission_bundle_sha256",
      "host_frozen",
      "host_state_archive_sha256",
      "irq_baseline_sha256",
      "irq_burn_in_passed",
      "irq_burn_in_receipt_sha256",
      "passed",
      "profile_sha256",
      "qualification_evidence",
      "release",
      "schema_version",
      "ship_evidence_eligible",
      "source_commit",
      "stage"
    ] | sort) and
    .schema_version == 1 and
    .release == "0.67.1" and
    .stage == "reference-campaign-host-admission" and
    (.campaign_id | test("^hc0671-[a-z0-9][a-z0-9-]{5,55}$")) and
    .source_commit == $source_commit and
    .profile_sha256 == $profile_sha256 and
    .host_admission_bundle_sha256 == $bundle_sha256 and
    (.host_state_archive_sha256 | test("^[0-9a-f]{64}$")) and
    (.irq_burn_in_receipt_sha256 | test("^[0-9a-f]{64}$")) and
    (.irq_baseline_sha256 | test("^[0-9a-f]{64}$")) and
    .host_frozen == true and
    .irq_burn_in_passed == true and
    .passed == true and
    .qualification_evidence == false and
    .bootstrap_evidence == false and
    .ship_evidence_eligible == false
  ' "$source_receipt" >/dev/null

test ! -e "$target_dir" || {
  echo "reference campaign admission target already exists" >&2
  exit 1
}
mkdir --parents "$target_dir"
cp --update=none --preserve=mode,timestamps "$source_receipt" "$target_receipt"
cp --update=none --preserve=mode,timestamps "$source_bundle" "$target_bundle"
test "$(sha256sum "$target_receipt" | awk '{print $1}')" = \
  "$(sha256sum "$source_receipt" | awk '{print $1}')"
test "$(sha256sum "$target_bundle" | awk '{print $1}')" = "$bundle_sha256"
chmod 0444 "$target_receipt" "$target_bundle"
echo "reference campaign host admission imported: $target_dir"
