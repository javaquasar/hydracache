"""Read-only Windows storage observations; writes only a new evidence directory."""
import argparse
import ctypes
from ctypes import wintypes
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parents[2]
SINCE = datetime.fromisoformat('2026-10-27T13:09:20+00:00').timestamp()
PS = r"""
$ErrorActionPreference='Stop'
$os=Get-CimInstance Win32_OperatingSystem
$pages=@(Get-CimInstance Win32_PageFileUsage | Select-Object Name,AllocatedBaseSize,CurrentUsage,PeakUsage)
$processes=@(Get-Process | Sort-Object PrivateMemorySize64 -Descending | Select-Object -First 12 Id,ProcessName,PrivateMemorySize64,WorkingSet64)
[pscustomobject]@{utc=[DateTime]::UtcNow.ToString('o');os=($os | Select-Object TotalVisibleMemorySize,FreePhysicalMemory,TotalVirtualMemorySize,FreeVirtualMemory);pagefiles=$pages;processes=$processes} | ConvertTo-Json -Depth 5 -Compress
"""


def now():
    return datetime.now(timezone.utc).isoformat()


def write(out, name, value):
    with (out / name).open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write('\n')


def allocated(path):
    """GetCompressedFileSize is a size query, not a compression operation."""
    dll = ctypes.WinDLL('kernel32', use_last_error=True)
    query = dll.GetCompressedFileSizeW
    query.argtypes = [wintypes.LPCWSTR, ctypes.POINTER(wintypes.DWORD)]
    query.restype = wintypes.DWORD
    high = wintypes.DWORD()
    ctypes.set_last_error(0)
    low = query(str(path), ctypes.byref(high))
    error = ctypes.get_last_error()
    if low == 0xffffffff and error:
        return dict(allocated_bytes=None, winerror=error)
    return dict(allocated_bytes=(high.value << 32) | low, winerror=None)


def file_row(path):
    row = dict(path=str(path), observed_utc=now())
    try:
        stat = path.stat(follow_symlinks=False)
        row.update(logical_bytes=stat.st_size, modified_ns=stat.st_mtime_ns,
                   created_ns=stat.st_birthtime_ns, attributes=stat.st_file_attributes,
                   **allocated(path))
    except OSError as error:
        row.update(error=type(error).__name__, winerror=error.winerror)
    return row


def snapshot():
    import winreg
    files = [Path(r'C:\pagefile.sys'), Path(r'C:\hiberfil.sys')]
    distros = []
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER,
                        r'Software\Microsoft\Windows\CurrentVersion\Lxss') as key:
        for index in range(winreg.QueryInfoKey(key)[0]):
            with winreg.OpenKey(key, winreg.EnumKey(key, index)) as child:
                name = winreg.QueryValueEx(child, 'DistributionName')[0]
                base = winreg.QueryValueEx(child, 'BasePath')[0]
                try:
                    vhd = winreg.QueryValueEx(child, 'VhdFileName')[0]
                except FileNotFoundError:
                    vhd = 'ext4.vhdx'
                distros.append(dict(name=name, base=base, vhd=vhd))
                files.append(Path(base) / vhd)
    return dict(utc=now(), volume_free_bytes=shutil.disk_usage(ROOT).free,
                volume_total_bytes=shutil.disk_usage(ROOT).total,
                files=[file_row(path) for path in files], distros=distros)


def scan(root, since=SINCE):
    """Metadata only, no content reads, no reparse traversal; totals are logical."""
    stack = [root]
    groups = {}
    errors = []
    skipped = []
    largest = []
    while stack:
        folder = stack.pop()
        try:
            with os.scandir(folder) as entries:
                for entry in entries:
                    try:
                        stat = entry.stat(follow_symlinks=False)
                        if getattr(stat, 'st_file_attributes', 0) & 0x400 or entry.is_symlink():
                            skipped.append(str(entry.path))
                            continue
                        if entry.is_dir(follow_symlinks=False):
                            stack.append(Path(entry.path))
                        elif entry.is_file(follow_symlinks=False):
                            relative = Path(entry.path).relative_to(root)
                            group = relative.parts[0] if len(relative.parts) > 1 else '<root-files>'
                            row = groups.setdefault(group, dict(files=0, logical_bytes=0,
                                created_since_files=0, created_since_logical_bytes=0,
                                modified_since_files=0, modified_since_logical_bytes=0))
                            row['files'] += 1
                            row['logical_bytes'] += stat.st_size
                            if getattr(stat, 'st_birthtime', stat.st_ctime) >= since:
                                row['created_since_files'] += 1
                                row['created_since_logical_bytes'] += stat.st_size
                            if stat.st_mtime >= since:
                                row['modified_since_files'] += 1
                                row['modified_since_logical_bytes'] += stat.st_size
                            largest.append((stat.st_size, str(relative), stat.st_mtime_ns))
                            if len(largest) > 64:
                                largest = sorted(largest, reverse=True)[:20]
                    except OSError as error:
                        errors.append(dict(path=str(entry.path), error=str(error)))
        except OSError as error:
            errors.append(dict(path=str(folder), error=str(error)))
    return dict(root=str(root), groups=groups, errors=errors, skipped_reparse=skipped,
                largest=sorted(largest, reverse=True)[:20], utc=now(), since_epoch=since,
                complete=not errors and not skipped, allocation_or_growth_measurement=False)


def capture_ps(out, label):
    # Open both raw streams before starting any CIM/process query.
    with (out / (label + '.stdout.log')).open('xb') as stdout, \
            (out / (label + '.stderr.log')).open('xb') as stderr:
        try:
            result = subprocess.run(['powershell.exe', '-NoLogo', '-NoProfile', '-NonInteractive',
                '-Command', PS], stdout=stdout, stderr=stderr, timeout=60, check=False)
            write(out, label + '.json', dict(exit=result.returncode, utc=now()))
        except BaseException as error:
            write(out, label + '.json', dict(error=type(error).__name__, reason=str(error), utc=now()))
            raise


def collect(out):
    out.mkdir(parents=True, exist_ok=False)
    write(out, 'intent.json', dict(utc=now(), root=str(ROOT), since_epoch=SINCE,
        source_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        read_only=True, system_changes=False, process_stops=False, gates_dispatched=False))
    try:
        if os.name != 'nt':
            raise ValueError('Windows only')
        write(out, 'before.json', snapshot())
        capture_ps(out, 'memory-before')
        write(out, 'target.json', scan(ROOT / 'target'))
        write(out, 'git-objects.json', scan(ROOT.parent / 'hydracache/.git/objects'))
        write(out, 'removals.json', {str(path): path.exists() for path in [
            ROOT.parent / 't067-rescope',
            *(ROOT.parent / name / 'target/debug' for name in (
                'hydra-moka-release', 'moka-post-removal-observer-pr', 'moka-upstream-prep'))]})
        capture_ps(out, 'memory-after')
        write(out, 'after.json', snapshot())
        write(out, 'result.json', dict(complete=True, gate_pass=False,
            causal_attribution_complete=False, utc=now(), process_stops=False))
    except BaseException as error:
        write(out, 'refusal.json', dict(type=type(error).__name__, reason=str(error), utc=now()))
        raise


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('out', type=Path)
    args = parser.parse_args()
    collect(args.out)
    print(json.dumps(dict(evidence=str(args.out), read_only=True, gate_dispatched=False)))
