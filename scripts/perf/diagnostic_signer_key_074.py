"""Separate local signer-build issuer; public selfcheck only, no receipt or job authority."""
import argparse
from contextlib import ExitStack
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import sys

PRIVATE = "signer-build-private.pem"
PUBLIC = "signer-build-public.json"
OLD_ROLES = frozenset({
    "c56f89595c45d0656225f1b8915babed046d278dc9ce1908c6bcb6e9c93e2fda",
    "39215088b0102dffb987b937efa9f49a6c5c7704192fa1f16b4e19ccad12c3ba",
    "e7bd73d40638095eb7fdb3d03eec3c36af688d185fd750b83fc3ee81683a2f59",
})
SPKI_PREFIX = bytes.fromhex("302a300506032b6570032100")
PROBE = b"hydracache-diagnostic-signer-build-key-selfcheck-074-v1\0not-a-build-receipt\n"


class Refused(ValueError):
    """Bounded diagnostics must never repeat subprocess output or key material."""


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


def openssl(arguments, descriptors, expected=0):
    try:
        result = subprocess.run(
            ["/usr/bin/openssl", *arguments], pass_fds=tuple(descriptors),
            stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            env={"PATH": "/usr/bin:/bin", "LANG": "C"}, timeout=15, check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        raise Refused("OpenSSL unavailable or timed out") from None
    if result.returncode != expected:
        raise Refused("OpenSSL operation refused")
    return result.stdout


def public_bytes(der):
    if len(der) != 44 or not der.startswith(SPKI_PREFIX):
        raise Refused("Exact Ed25519 public encoding required")
    return der[len(SPKI_PREFIX):]


def selection(public, signature, uid):
    if len(public) != 32 or len(signature) != 64 or public.hex() in OLD_ROLES:
        raise Refused("Invalid public selfcheck size")
    return {
        "schema_version": "diagnostic-signer-build-issuer-selection-074-v1",
        "algorithm": "Ed25519", "purpose": "diagnostic-signer-build-receipt-only",
        "public_key_hex": public.hex(), "public_key_sha256": hashlib.sha256(public).hexdigest(),
        "selfcheck_message_hex": PROBE.hex(), "selfcheck_signature_hex": signature.hex(),
        "storage_uid": uid, "directory_mode": "0700", "private_key_mode": "0600",
        "storage_format": "unencrypted-pkcs8-pem", "signer_build_statement_sha256": None,
        "public_pin_approved": False, "production_trust_enrolled": False,
        "signer_build_receipt_signed": False, "workflow_installed": False,
        "admission_allowed": False,
    }


def platform_check():
    if sys.platform != "linux" or os.geteuid() == 0:
        raise Refused("Nonroot local Linux account required")


def directory(path):
    """Descriptor-relative no-follow walk; root/current owner, no writable peers, no Git."""
    platform_check()
    import pwd
    raw = os.fspath(path)
    if not raw.startswith("/") or any(part in ("", ".", "..") for part in raw.split("/")[1:]):
        raise Refused("Absolute canonical Linux directory required")
    home = Path(pwd.getpwuid(os.geteuid()).pw_dir)
    if not Path(raw).is_relative_to(home):
        raise Refused("Key custody must stay under the local Linux account home")
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
    fd = os.open("/", flags)
    try:
        for part in [None, *Path(raw).parts[1:]]:
            if part is not None:
                child = os.open(part, flags, dir_fd=fd)
                os.close(fd)
                fd = child
            info = os.fstat(fd)
            if not stat.S_ISDIR(info.st_mode) or info.st_uid not in (0, os.geteuid()) or info.st_mode & 0o022:
                raise Refused("Unsafe key-directory ancestry")
            try:
                os.stat(".git", dir_fd=fd, follow_symlinks=False)
            except FileNotFoundError:
                pass
            else:
                raise Refused("Git checkout cannot contain issuer custody")
        return fd
    except BaseException:
        os.close(fd)
        raise


def check_file(fd, mode, budget):
    info = os.fstat(fd)
    if (not stat.S_ISREG(info.st_mode) or stat.S_IMODE(info.st_mode) != mode
            or info.st_uid != os.geteuid() or info.st_nlink != 1 or not 0 < info.st_size <= budget):
        raise Refused("Unsafe or incomplete key-custody file")
    return info


def stable_metadata(info):
    # Reading may legitimately change atime; compare exact mutation/identity fields.
    return (info.st_dev, info.st_ino, info.st_mode, info.st_uid, info.st_gid,
            info.st_nlink, info.st_size, info.st_mtime_ns, info.st_ctime_ns)


def same_named(directory_fd, name, fd):
    named = os.stat(name, dir_fd=directory_fd, follow_symlinks=False)
    original = os.fstat(fd)
    if (named.st_dev, named.st_ino) != (original.st_dev, original.st_ino):
        raise Refused("Original key-custody object replaced")


def same_directory(path, fd):
    fresh = directory(path)
    try:
        before, now = os.fstat(fd), os.fstat(fresh)
        if (before.st_dev, before.st_ino) != (now.st_dev, now.st_ino):
            raise Refused("Original key directory replaced")
        if stat.S_IMODE(now.st_mode) != 0o700 or now.st_uid != os.geteuid():
            raise Refused("Private key directory required")
    finally:
        os.close(fresh)


def fd_path(fd):
    return f"/proc/self/fd/{fd}"


def write_all(fd, data):
    remaining = memoryview(data)
    while remaining:
        count = os.write(fd, remaining)
        if count <= 0:
            raise Refused("Key-custody write made no progress")
        remaining = remaining[count:]


def memory_file(stack, data):
    import fcntl
    fd = os.memfd_create("hydracache-public-selfcheck", os.MFD_CLOEXEC | os.MFD_ALLOW_SEALING)
    stack.callback(os.close, fd)
    write_all(fd, data)
    os.lseek(fd, 0, os.SEEK_SET)
    fcntl.fcntl(fd, fcntl.F_ADD_SEALS, fcntl.F_SEAL_WRITE | fcntl.F_SEAL_GROW | fcntl.F_SEAL_SHRINK | fcntl.F_SEAL_SEAL)
    return fd


def derive_and_check(key_fd):
    before = check_file(key_fd, 0o600, 4096)
    der = openssl(["pkey", "-in", fd_path(key_fd), "-pubout", "-outform", "DER"], (key_fd,))
    public = public_bytes(der)
    with ExitStack() as stack:
        message = memory_file(stack, PROBE)
        signature = openssl(["pkeyutl", "-sign", "-rawin", "-inkey", fd_path(key_fd), "-in", fd_path(message)], (key_fd, message))
        if len(signature) != 64:
            raise Refused("Invalid Ed25519 selfcheck signature size")
        public_fd = memory_file(stack, der)
        signature_fd = memory_file(stack, signature)
        command = ["pkeyutl", "-verify", "-rawin", "-pubin", "-keyform", "DER", "-inkey", fd_path(public_fd), "-sigfile", fd_path(signature_fd)]
        openssl([*command, "-in", fd_path(message)], (public_fd, signature_fd, message))
        changed = memory_file(stack, PROBE + b"changed")
        openssl([*command, "-in", fd_path(changed)], (public_fd, signature_fd, changed), expected=1)
    if stable_metadata(os.fstat(key_fd)) != stable_metadata(before):
        raise Refused("Original private key metadata drifted")
    return selection(public, signature, os.geteuid())


def generate(path):
    platform_check()
    path = Path(path)
    with ExitStack() as stack:
        parent = directory(path.parent)
        stack.callback(os.close, parent)
        os.mkdir(path.name, 0o700, dir_fd=parent)  # Existing targets always refuse.
        fd = directory(path)
        stack.callback(os.close, fd)
        same_directory(path, fd)
        key = os.open(PRIVATE, os.O_RDWR | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=fd)
        stack.callback(os.close, key)
        if openssl(["genpkey", "-algorithm", "ED25519", "-out", fd_path(key)], (key,)):
            raise Refused("Unexpected key-generation output")
        os.fsync(key)
        report = derive_and_check(key)
        same_named(fd, PRIVATE, key)
        public = os.open(PUBLIC, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o400, dir_fd=fd)
        stack.callback(os.close, public)
        write_all(public, canonical(report))
        os.fsync(public)
        check_file(public, 0o400, 4096)
        same_named(fd, PUBLIC, public)
        same_directory(path, fd)
        os.fsync(fd)
        os.fsync(parent)
        return report  # Public selfcheck only; no receipt, trust enrollment or job authority.


def unique(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise Refused("Duplicate public selection field")
        result[key] = value
    return result


def read_public(fd):
    before = check_file(fd, 0o400, 4096)
    raw = os.pread(fd, 4097, 0)
    try:
        report = json.loads(raw, object_pairs_hook=unique)
        expected = selection(bytes.fromhex(report["public_key_hex"]),
                             bytes.fromhex(report["selfcheck_signature_hex"]), os.geteuid())
        if raw != canonical(expected):
            raise Refused("Exact unapproved signer public selection required")
    except (KeyError, TypeError, ValueError):
        raise Refused("Invalid signer public selection") from None
    if stable_metadata(os.fstat(fd)) != stable_metadata(before):
        raise Refused("Original public selection metadata drifted")
    return raw, before


def inspect(path):
    platform_check()
    path = Path(path)
    with ExitStack() as stack:
        fd = directory(path)
        stack.callback(os.close, fd)
        same_directory(path, fd)
        # Public role and closed shape are checked before ANY private key open.
        public = os.open(PUBLIC, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC, dir_fd=fd)
        stack.callback(os.close, public)
        raw, before = read_public(public)
        key = os.open(PRIVATE, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC, dir_fd=fd)
        stack.callback(os.close, key)
        report = derive_and_check(key)
        if (raw != canonical(report) or os.pread(public, 4097, 0) != raw
                or stable_metadata(os.fstat(public)) != stable_metadata(before)):
            raise Refused("Public signer selection differs from original private key")
        same_named(fd, PRIVATE, key)
        same_named(fd, PUBLIC, public)
        same_directory(path, fd)
        return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("generate", "inspect"))
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    try:
        report = generate(args.directory) if args.operation == "generate" else inspect(args.directory)
        sys.stdout.buffer.write(canonical(report))
    except (Refused, OSError):
        # Never print key-bearing tool output, tracebacks, argv or environment.
        print("signer-build issuer operation refused; retain state, do not overwrite or repair", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
