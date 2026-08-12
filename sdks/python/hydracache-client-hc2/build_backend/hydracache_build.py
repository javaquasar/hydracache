"""Minimal deterministic PEP 517 backend for the pure-Python HC/2 SDK."""

from __future__ import annotations

import base64
import csv
import hashlib
import io
from pathlib import Path
import zipfile

NAME = "hydracache-client-hc2"
VERSION = "0.68.0a1"
DIST = "hydracache_client_hc2"


def build_wheel(wheel_directory, config_settings=None, metadata_directory=None):
    del config_settings, metadata_directory
    root = Path(__file__).resolve().parents[1]
    wheel_name = f"{DIST}-{VERSION}-py3-none-any.whl"
    output = Path(wheel_directory) / wheel_name
    output.parent.mkdir(parents=True, exist_ok=True)
    dist_info = f"{DIST}-{VERSION}.dist-info"
    entries = {}
    for package in ("hydracache_hc2", "hydracache_hc2_generated"):
        for source in sorted((root / "src" / package).rglob("*")):
            if source.is_file() and source.suffix in {".py", ".pyi", ".json"}:
                entries[source.relative_to(root / "src").as_posix()] = source.read_bytes()
    entries[f"{dist_info}/METADATA"] = _metadata().encode()
    entries[f"{dist_info}/WHEEL"] = (
        "Wheel-Version: 1.0\nGenerator: hydracache-build-v1\n"
        "Root-Is-Purelib: true\nTag: py3-none-any\n"
    ).encode()
    entries[f"{dist_info}/top_level.txt"] = b"hydracache_hc2\nhydracache_hc2_generated\n"
    record = []
    with zipfile.ZipFile(output, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, payload in sorted(entries.items()):
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            archive.writestr(info, payload)
            digest = base64.urlsafe_b64encode(hashlib.sha256(payload).digest()).rstrip(b"=").decode()
            record.append((name, f"sha256={digest}", str(len(payload))))
        record_name = f"{dist_info}/RECORD"
        record.append((record_name, "", ""))
        buffer = io.StringIO(newline="")
        csv.writer(buffer, lineterminator="\n").writerows(record)
        info = zipfile.ZipInfo(record_name, (1980, 1, 1, 0, 0, 0))
        info.compress_type = zipfile.ZIP_DEFLATED
        info.external_attr = 0o644 << 16
        archive.writestr(info, buffer.getvalue().encode())
    return wheel_name


def prepare_metadata_for_build_wheel(metadata_directory, config_settings=None):
    del config_settings
    dist_info = f"{DIST}-{VERSION}.dist-info"
    output = Path(metadata_directory) / dist_info
    output.mkdir(parents=True, exist_ok=True)
    (output / "METADATA").write_text(_metadata(), encoding="utf-8", newline="\n")
    (output / "WHEEL").write_text(
        "Wheel-Version: 1.0\nGenerator: hydracache-build-v1\n"
        "Root-Is-Purelib: true\nTag: py3-none-any\n",
        encoding="utf-8",
        newline="\n",
    )
    return dist_info


def _metadata():
    return """Metadata-Version: 2.4
Name: hydracache-client-hc2
Version: 0.68.0a1
Summary: Async Python client for the generated HydraCache HC/2 client plane.
License-Expression: Apache-2.0
Requires-Python: >=3.10
Requires-Dist: grpcio>=1.76,<2
Requires-Dist: protobuf>=6.33.4,<7
Requires-Dist: typing-extensions>=4.15,<5; python_version < '3.11'
Project-URL: Homepage, https://github.com/javaquasar/hydracache
Project-URL: Repository, https://github.com/javaquasar/hydracache
"""
