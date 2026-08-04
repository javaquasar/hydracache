#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
export LC_ALL=C

readonly source_root="/repo"
readonly work_root="/work/repo"
readonly state_dir="/var/lib/hydracache-perf/local-systemd-state"
readonly plan_only_dir="/var/lib/hydracache-perf/local-systemd-plan"
readonly profile="$work_root/local-systemd-profile.json"

fail() {
  echo "local systemd orchestration smoke: $*" >&2
  exit 1
}

expect_failure() {
  local label="$1"
  shift
  if "$@" >/tmp/expected-failure.stdout 2>/tmp/expected-failure.stderr; then
    fail "negative canary unexpectedly passed: $label"
  fi
  printf 'negative canary rejected: %s\n' "$label"
}

cleanup() {
  if test -f "$state_dir/applied.json" && test ! -f "$state_dir/restored.json"; then
    "$work_root/scripts/perf/reference-host-tuning.sh" restore \
      --profile "$profile" --state-dir "$state_dir" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

systemd_state=""
for _attempt in $(seq 1 60); do
  systemd_state="$(systemctl is-system-running 2>/dev/null || true)"
  case "$systemd_state" in
    running|degraded)
      break
      ;;
  esac
  sleep 1
done
case "$systemd_state" in
  running|degraded) ;;
  *) fail "systemd did not become usable after 60 seconds: $systemd_state" ;;
esac

rm --recursive --force -- /work/repo
mkdir --parents /work/repo /work/shims
(cd "$source_root" && tar --exclude=.git --exclude=target --create --file=- .) |
  tar --extract --file=- --directory=/work/repo

cat >/work/shims/uname <<'EOF'
#!/usr/bin/env bash
case "${1:-}" in
  -s) echo Linux ;;
  -m) echo x86_64 ;;
  -r) echo 6.8.0-999-generic ;;
  *) exec /usr/bin/uname "$@" ;;
esac
EOF
cat >/work/shims/systemd-detect-virt <<'EOF'
#!/usr/bin/env bash
if test "${1:-}" = --quiet; then exit 1; fi
echo none
EOF
cat >/work/shims/stat <<'EOF'
#!/usr/bin/env bash
if test "$*" = "--file-system --format=%T /sys/fs/cgroup"; then
  echo cgroup2fs
  exit 0
fi
exec /usr/bin/stat "$@"
EOF
cat >/work/shims/lscpu <<'EOF'
#!/usr/bin/env bash
case "${1:-}" in
  --parse=SOCKET,CORE)
    printf '%s\n' '# Socket,Core' 0,0 0,1 0,2 0,3 0,4 0,5 0,6 0,7
    ;;
  --json)
    printf '%s\n' '{"lscpu":[{"field":"Architecture:","data":"x86_64"}]}'
    ;;
  *) exec /usr/bin/lscpu "$@" ;;
esac
EOF
cat >/work/shims/lsblk <<'EOF'
#!/usr/bin/env bash
if printf '%s\n' "$*" | grep --quiet -- '--json'; then
  printf '%s\n' '{"blockdevices":[{"name":"nvme0n1","kname":"nvme0n1","path":"/dev/nvme0n1","type":"disk","tran":"nvme","size":1099511627776,"rota":false,"mountpoints":[null]}]}'
else
  echo nvme
fi
EOF
cat >/work/shims/awk <<'EOF'
#!/usr/bin/env bash
if printf '%s\n' "$*" | grep --quiet 'MemTotal'; then
  echo 68719476736
  exit 0
fi
exec /usr/bin/gawk "$@"
EOF
cat >/work/shims/sysctl <<'EOF'
#!/usr/bin/env bash
if test "${1:-}" = --values && test "$#" -eq 2; then
  echo 1
  exit 0
fi
exec /usr/sbin/sysctl "$@"
EOF
chmod 0755 /work/shims/*
export PATH="/work/shims:$PATH"
cd "$work_root"

cat >scripts/perf/provision-reference-isolation.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
test "${1:-}" = verify
echo 'local fixture: isolation verifier boundary exercised'
EOF
cat >scripts/perf/audit-reference-host.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
test "${1:-}" = --mode
test "${2:-}" = provisioned
echo 'local fixture: provisioned audit boundary exercised'
EOF
chmod 0755 scripts/perf/provision-reference-isolation.sh scripts/perf/audit-reference-host.sh

jq '
  .service_policy.protected_units = ["hc-remote.service", "hc-time.service"] |
  .service_policy.required_active_groups = [
    {id: "remote-access", one_of: ["hc-remote.service"]},
    {id: "time-synchronization", one_of: ["hc-time.service"]}
  ] |
  .service_policy.disable_if_present = ["hc-disable.service"] |
  .service_policy.mask_if_present = ["hc-mask.service"] |
  .service_policy.require_inactive_if_present = ["hc-inactive.service"] |
  .service_policy.report_only_candidates = ["hc-report.service"]
' docs/testing/perf-host-profiles/ubuntu-24.04-reference-v1.json >"$profile"

git init --quiet
git config user.email local-orchestration@invalid.example
git config user.name hydracache-local-orchestration
git add --all
git commit --quiet --message fixture
fixture_commit="$(git rev-parse HEAD)"
readonly fixture_commit

make_service() {
  local unit="$1"
  cat >"/usr/lib/systemd/system/$unit" <<EOF
[Unit]
Description=HydraCache local orchestration fixture $unit
[Service]
Type=simple
ExecStart=/bin/sleep infinity
[Install]
WantedBy=multi-user.target
EOF
}
for unit in hc-remote.service hc-time.service hc-disable.service hc-mask.service \
  hc-inactive.service hc-report.service; do
  make_service "$unit"
  systemctl enable "$unit" >/dev/null
  systemctl start "$unit"
done

useradd --system --create-home github-runner
mkdir --parents /opt/actions-runner /etc/hydracache-perf
cat >/opt/actions-runner/.runner <<'EOF'
{"agentName":"hydracache-perf-v1","gitHubUrl":"https://github.com/javaquasar/hydracache","workFolder":"_work"}
EOF
cat >/etc/hydracache-perf/runner-contract.json <<'EOF'
{"schema_version":1,"repository":"javaquasar/hydracache","runner_name":"hydracache-perf-v1","labels":["self-hosted","linux","x64","hydracache-perf-v1"],"service_user":"github-runner"}
EOF
chown --recursive github-runner:github-runner /opt/actions-runner
cat >/etc/systemd/system/actions.runner.javaquasar-hydracache.hydracache-perf-v1.service <<'EOF'
[Unit]
Description=HydraCache local runner fixture
[Service]
User=github-runner
ExecStart=/bin/sleep infinity
EOF
systemctl daemon-reload
systemctl enable actions.runner.javaquasar-hydracache.hydracache-perf-v1.service >/dev/null

scripts/perf/verify-runner-service.sh --expected-label hydracache-perf-v1
scripts/perf/runner-service.sh online
expect_failure tuning-while-runner-online \
  scripts/perf/reference-host-tuning.sh apply --profile "$profile" --state-dir "$state_dir"
scripts/perf/runner-service.sh offline

scripts/perf/reference-host-tuning.sh plan --profile "$profile" --state-dir "$plan_only_dir"
jq --exit-status '
  .actions | length == 4 and
  any(.[]; .unit == "hc-disable.service" and .active_state == "active") and
  any(.[]; .unit == "hc-mask.service" and .unit_file_state == "enabled")
' "$plan_only_dir/plan.json" >/dev/null

scripts/perf/reference-host-tuning.sh apply --profile "$profile" --state-dir "$state_dir"
test "$(systemctl is-active hc-disable.service || true)" = inactive
test "$(systemctl is-enabled hc-disable.service || true)" = disabled
test "$(systemctl is-active hc-mask.service || true)" = inactive
test "$(systemctl is-enabled hc-mask.service || true)" = masked
test "$(systemctl is-active hc-inactive.service || true)" = inactive
scripts/perf/reference-host-tuning.sh verify --profile "$profile" --state-dir "$state_dir"
expect_failure repeated-apply \
  scripts/perf/reference-host-tuning.sh apply --profile "$profile" --state-dir "$state_dir"

mkdir --parents target/test-evidence/0.67.1
jq --null-input --arg commit "$fixture_commit" \
  '{schema_version: 4, source_commit: $commit, local_fixture: true,
    ship_evidence_eligible: false}' \
  >target/test-evidence/0.67.1/runner-provisioned.json
chmod 0444 target/test-evidence/0.67.1/runner-provisioned.json
scripts/perf/reference-host-tuning.sh freeze --profile "$profile" --state-dir "$state_dir"
scripts/perf/check-reference-host-freeze.sh --profile "$profile" --state-dir "$state_dir"

systemctl start hc-disable.service
expect_failure frozen-service-drift \
  scripts/perf/check-reference-host-freeze.sh --profile "$profile" --state-dir "$state_dir"
systemctl stop hc-disable.service
scripts/perf/check-reference-host-freeze.sh --profile "$profile" --state-dir "$state_dir"

scripts/perf/reference-host-tuning.sh restore --profile "$profile" --state-dir "$state_dir"
for unit in hc-disable.service hc-mask.service hc-inactive.service; do
  test "$(systemctl is-active "$unit")" = active
  test "$(systemctl is-enabled "$unit")" = enabled
done
expect_failure repeated-restore \
  scripts/perf/reference-host-tuning.sh restore --profile "$profile" --state-dir "$state_dir"
mkdir --parents /var/lib/hydracache-perf/incomplete-state
expect_failure incomplete-restore \
  scripts/perf/reference-host-tuning.sh restore \
    --profile "$profile" --state-dir /var/lib/hydracache-perf/incomplete-state

trap - EXIT
printf 'systemd runner and plan/apply/verify/freeze/restore checks: OK\n'
