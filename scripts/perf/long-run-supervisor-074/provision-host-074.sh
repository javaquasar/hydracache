#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

SERVICE=hydracache-performance-supervisor-074.service
SUPERVISOR_USER=hydracache-perf
CLIENT_GROUP=hydracache-perf-client
INSTALL_BINARY=/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074
INSTALL_CONFIG=/etc/hydracache-perf/supervisor-074.toml
INSTALL_SERVICE=/etc/systemd/system/$SERVICE
INSTALL_SYSUSERS=/etc/sysusers.d/hydracache-performance-074.conf
INSTALL_TMPFILES=/etc/tmpfiles.d/hydracache-performance-074.conf
RECEIPT=/var/lib/hydracache-performance/provisioning-receipt-074.json

die() {
  printf 'provision-host-074: %s\n' "$*" >&2
  exit 2
}

usage() {
  die "usage: $0 --bundle-dir DIR --source-commit SHA --repository-id ID --actor-id ID --runner-user NAME --runner-uid UID --runner-gid GID"
}

bundle_dir=
source_commit=
repository_id=
actor_id=
runner_user=
runner_uid=
runner_gid=
while (( $# > 0 )); do
  case "$1" in
    --bundle-dir) bundle_dir=${2-}; shift 2 ;;
    --source-commit) source_commit=${2-}; shift 2 ;;
    --repository-id) repository_id=${2-}; shift 2 ;;
    --actor-id) actor_id=${2-}; shift 2 ;;
    --runner-user) runner_user=${2-}; shift 2 ;;
    --runner-uid) runner_uid=${2-}; shift 2 ;;
    --runner-gid) runner_gid=${2-}; shift 2 ;;
    *) usage ;;
  esac
done

(( EUID == 0 )) || die "root is required"
[[ "$source_commit" =~ ^[0-9a-f]{40}$ ]] || die "invalid source commit"
[[ "$repository_id" =~ ^[1-9][0-9]*$ ]] || die "invalid repository id"
[[ "$actor_id" =~ ^[1-9][0-9]*$ ]] || die "invalid actor id"
[[ "$runner_user" =~ ^[a-z_][a-z0-9_-]{0,31}$ ]] || die "invalid runner user"
[[ "$runner_uid" =~ ^[1-9][0-9]*$ ]] || die "invalid runner uid"
[[ "$runner_gid" =~ ^[1-9][0-9]*$ ]] || die "invalid runner gid"
[[ "$bundle_dir" = /* && -d "$bundle_dir" && ! -L "$bundle_dir" ]] || die "unsafe bundle directory"
bundle_dir=$(realpath -e -- "$bundle_dir")

required=(
  bundle.sha256
  hydracache-long-run-supervisor-074
  hydracache-performance-074.sysusers.conf
  hydracache-performance-074.tmpfiles.conf
  hydracache-performance-supervisor-074.service
  provision-host-074.sh
  source-commit.txt
  verification-key.hex
)
mapfile -t actual < <(find "$bundle_dir" -mindepth 1 -maxdepth 1 -printf '%f\n' | LC_ALL=C sort)
mapfile -t expected < <(printf '%s\n' "${required[@]}" | LC_ALL=C sort)
[[ "${actual[*]}" = "${expected[*]}" ]] || die "bundle file set differs from the reviewed contract"
for name in "${required[@]}"; do
  [[ -f "$bundle_dir/$name" && ! -L "$bundle_dir/$name" ]] || die "unsafe bundle member: $name"
done
digest_members=(
  hydracache-long-run-supervisor-074
  hydracache-performance-074.sysusers.conf
  hydracache-performance-074.tmpfiles.conf
  hydracache-performance-supervisor-074.service
  provision-host-074.sh
  source-commit.txt
  verification-key.hex
)
mapfile -t actual_digest_members < <(sed -nE 's/^[0-9a-f]{64}  ([A-Za-z0-9._-]+)$/\1/p' "$bundle_dir/bundle.sha256" | LC_ALL=C sort)
mapfile -t expected_digest_members < <(printf '%s\n' "${digest_members[@]}" | LC_ALL=C sort)
[[ "${actual_digest_members[*]}" = "${expected_digest_members[*]}" ]] || die "bundle digest member set differs"
(
  cd "$bundle_dir"
  sha256sum --strict --check bundle.sha256 >/dev/null
) || die "bundle digest verification failed"
[[ "$(tr -d '\r\n' < "$bundle_dir/source-commit.txt")" = "$source_commit" ]] || die "bundle source commit differs"
verification_key=$(tr -d '\r\n' < "$bundle_dir/verification-key.hex")
[[ "$verification_key" =~ ^[0-9a-f]{64}$ ]] || die "invalid public verification key"

command -v systemctl >/dev/null || die "systemctl is unavailable"
command -v systemd-sysusers >/dev/null || die "systemd-sysusers is unavailable"
command -v systemd-tmpfiles >/dev/null || die "systemd-tmpfiles is unavailable"
command -v usermod >/dev/null || die "usermod is unavailable"
[[ "$(getent passwd "$runner_user" | cut -d: -f3)" = "$runner_uid" ]] || die "runner user/uid mismatch"
[[ "$(getent passwd "$runner_user" | cut -d: -f4)" = "$runner_gid" ]] || die "runner user/gid mismatch"
pid1=$(</proc/1/comm)
[[ "$pid1" = systemd ]] || die "PID 1 is not systemd"
systemctl is-system-running --quiet || [[ "$(systemctl is-system-running)" = degraded ]] || die "systemd is not operational"

atomic_install() {
  local source=$1 destination=$2 mode=$3 owner=$4 group=$5 parent temporary
  parent=$(dirname -- "$destination")
  install -d -m 0755 -o root -g root "$parent"
  temporary=$(mktemp "$parent/.hydracache-074.XXXXXX")
  if ! install -m "$mode" -o "$owner" -g "$group" -- "$source" "$temporary"; then
    rm -f -- "$temporary"
    return 1
  fi
  if ! mv -f -- "$temporary" "$destination"; then
    rm -f -- "$temporary"
    return 1
  fi
}

make_config() {
  local destination=$1 client_gid=$2
  umask 077
  printf '%s\n' \
    'schema_version = 1' \
    'socket_path = "/run/hydracache-perf/supervisor-v1.sock"' \
    'campaign_root = "/var/lib/hydracache-performance/campaigns"' \
    'staging_root = "/var/lib/hydracache-performance/staging"' \
    'seal_root = "/var/lib/hydracache-performance/seals"' \
    'socket_mode = 432' \
    "expected_repository_id = $repository_id" \
    "allowed_actor_ids = [$actor_id]" \
    "allowed_client_uids = [$runner_uid]" \
    "required_client_gid = $client_gid" \
    "verification_key_hex = \"$verification_key\"" > "$destination"
}

files_match_active_install() {
  local candidate_config=$1
  cmp -s "$bundle_dir/hydracache-long-run-supervisor-074" "$INSTALL_BINARY" &&
    cmp -s "$bundle_dir/hydracache-performance-supervisor-074.service" "$INSTALL_SERVICE" &&
    cmp -s "$bundle_dir/hydracache-performance-074.sysusers.conf" "$INSTALL_SYSUSERS" &&
    cmp -s "$bundle_dir/hydracache-performance-074.tmpfiles.conf" "$INSTALL_TMPFILES" &&
    cmp -s "$candidate_config" "$INSTALL_CONFIG"
}

already_active=false
if systemctl is-active --quiet "$SERVICE"; then
  already_active=true
  getent group "$CLIENT_GROUP" >/dev/null || die "active service lacks the frozen client group"
  client_gid=$(getent group "$CLIENT_GROUP" | cut -d: -f3)
  [[ "$client_gid" =~ ^[1-9][0-9]*$ ]] || die "invalid client group gid"
  candidate_config=$(mktemp)
  trap 'rm -f -- "$candidate_config"' EXIT
  make_config "$candidate_config" "$client_gid"
  files_match_active_install "$candidate_config" || die "refusing to replace a differing active installation"
fi

if [[ "$already_active" = false ]]; then
  systemd-sysusers "$bundle_dir/hydracache-performance-074.sysusers.conf"
  getent passwd "$SUPERVISOR_USER" >/dev/null || die "supervisor user was not created"
  getent group "$CLIENT_GROUP" >/dev/null || die "client group was not created"
  client_gid=$(getent group "$CLIENT_GROUP" | cut -d: -f3)
  [[ "$client_gid" =~ ^[1-9][0-9]*$ ]] || die "invalid client group gid"
  candidate_config=$(mktemp)
  trap 'rm -f -- "$candidate_config"' EXIT
  make_config "$candidate_config" "$client_gid"
  "$bundle_dir/hydracache-long-run-supervisor-074" validate-production-config "$candidate_config"

  atomic_install "$bundle_dir/hydracache-long-run-supervisor-074" "$INSTALL_BINARY" 0755 root root
  atomic_install "$bundle_dir/hydracache-performance-supervisor-074.service" "$INSTALL_SERVICE" 0644 root root
  atomic_install "$bundle_dir/hydracache-performance-074.sysusers.conf" "$INSTALL_SYSUSERS" 0644 root root
  atomic_install "$bundle_dir/hydracache-performance-074.tmpfiles.conf" "$INSTALL_TMPFILES" 0644 root root
  atomic_install "$candidate_config" "$INSTALL_CONFIG" 0600 root root
  systemd-tmpfiles --create "$INSTALL_TMPFILES"
  systemctl daemon-reload
  systemctl enable --now "$SERVICE"
fi

usermod -a -G "$CLIENT_GROUP" "$runner_user"
id -nG "$runner_user" | tr ' ' '\n' | grep -Fx "$CLIENT_GROUP" >/dev/null || die "runner group admission failed"
"$INSTALL_BINARY" validate-production-config "$INSTALL_CONFIG"
systemctl is-active --quiet "$SERVICE" || die "supervisor service is not active"
[[ "$(systemctl show "$SERVICE" --property=Type --value)" = notify ]] || die "unexpected service type"
[[ "$(systemctl show "$SERVICE" --property=User --value)" = root ]] || die "unexpected service user"
[[ "$(systemctl show "$SERVICE" --property=NoNewPrivileges --value)" = yes ]] || die "NoNewPrivileges is not active"
[[ "$(systemctl show "$SERVICE" --property=ProtectSystem --value)" = strict ]] || die "ProtectSystem is not strict"
[[ "$(systemctl show "$SERVICE" --property=ProtectHome --value)" = yes ]] || die "ProtectHome is not active"
[[ "$(systemctl show "$SERVICE" --property=ProtectControlGroups --value)" = yes ]] || die "ProtectControlGroups is not active"
[[ "$(systemctl show "$SERVICE" --property=RestrictAddressFamilies --value)" = AF_UNIX ]] || die "address families differ"
[[ "$(systemctl show "$SERVICE" --property=RuntimeDirectoryMode --value)" = 0711 ]] || die "runtime directory mode differs"

socket=/run/hydracache-perf/supervisor-v1.sock
[[ -S "$socket" && ! -L "$socket" ]] || die "supervisor socket is absent"
[[ "$(stat -c '%a' "$socket")" = 660 ]] || die "supervisor socket mode differs"
[[ "$(stat -c '%u' "$socket")" = 0 ]] || die "supervisor socket owner differs"
[[ "$(stat -c '%g' "$socket")" = "$client_gid" ]] || die "supervisor socket group differs"
[[ "$(stat -c '%a:%u:%g' "$INSTALL_BINARY")" = 755:0:0 ]] || die "installed binary metadata differs"
[[ "$(stat -c '%a:%u:%g' "$INSTALL_CONFIG")" = 600:0:0 ]] || die "installed config metadata differs"

unit_properties=$(mktemp)
trap 'rm -f -- "$candidate_config" "$unit_properties"' EXIT
systemctl show "$SERVICE" \
  --property=Type,User,Group,NoNewPrivileges,ProtectSystem,ProtectHome,ProtectControlGroups,RestrictAddressFamilies,RuntimeDirectoryMode,KillMode,FragmentPath,ControlGroup \
  --no-pager > "$unit_properties"

binary_sha=$(sha256sum "$INSTALL_BINARY" | cut -d' ' -f1)
config_sha=$(sha256sum "$INSTALL_CONFIG" | cut -d' ' -f1)
service_sha=$(sha256sum "$INSTALL_SERVICE" | cut -d' ' -f1)
sysusers_sha=$(sha256sum "$INSTALL_SYSUSERS" | cut -d' ' -f1)
tmpfiles_sha=$(sha256sum "$INSTALL_TMPFILES" | cut -d' ' -f1)
key_sha=$(printf '%s' "$verification_key" | sha256sum | cut -d' ' -f1)
unit_properties_sha=$(sha256sum "$unit_properties" | cut -d' ' -f1)
machine_id_sha=$(tr -d '\r\n' < /etc/machine-id | sha256sum | cut -d' ' -f1)
boot_id_sha=$(tr -d '\r\n' < /proc/sys/kernel/random/boot_id | sha256sum | cut -d' ' -f1)
supervisor_uid=$(getent passwd "$SUPERVISOR_USER" | cut -d: -f3)
created_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)

receipt_temp=$(mktemp /var/lib/hydracache-performance/.provisioning-receipt-074.XXXXXX)
trap 'rm -f -- "$candidate_config" "$unit_properties" "$receipt_temp"' EXIT
printf '%s\n' \
  '{' \
  '  "schema_version": "hydracache-w11-host-provisioning-v1",' \
  "  \"source_commit\": \"$source_commit\"," \
  "  \"created_at_utc\": \"$created_at\"," \
  '  "mutation_performed": true,' \
  "  \"repository_id\": $repository_id," \
  "  \"allowed_actor_ids\": [$actor_id]," \
  "  \"runner_uid\": $runner_uid," \
  "  \"runner_gid\": $runner_gid," \
  "  \"supervisor_uid\": $supervisor_uid," \
  "  \"client_gid\": $client_gid," \
  '  "runner_group_database_membership": true,' \
  '  "runner_process_group_refresh_may_be_required": true,' \
  '  "service_active": true,' \
  '  "socket_mode": 432,' \
  "  \"binary_sha256\": \"$binary_sha\"," \
  "  \"config_sha256\": \"$config_sha\"," \
  "  \"service_sha256\": \"$service_sha\"," \
  "  \"sysusers_sha256\": \"$sysusers_sha\"," \
  "  \"tmpfiles_sha256\": \"$tmpfiles_sha\"," \
  "  \"verification_key_sha256\": \"$key_sha\"," \
  "  \"unit_properties_sha256\": \"$unit_properties_sha\"," \
  "  \"machine_id_sha256\": \"$machine_id_sha\"," \
  "  \"boot_id_sha256\": \"$boot_id_sha\"" \
  '}' > "$receipt_temp"
python3 -m json.tool "$receipt_temp" >/dev/null
chmod 0444 "$receipt_temp"
chown root:root "$receipt_temp"
mv -f -- "$receipt_temp" "$RECEIPT"
printf '%s\n' "$RECEIPT"
