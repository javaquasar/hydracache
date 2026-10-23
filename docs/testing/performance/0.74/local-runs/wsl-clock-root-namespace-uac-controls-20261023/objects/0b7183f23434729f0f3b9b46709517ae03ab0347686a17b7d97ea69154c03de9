# Journaling setup is the only work before logs; no elevation or live preflight here.
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$journal = Join-Path $root 'target/wsl-clock-root-namespace-logged-20261023'
$exitCode = 1
$stage = 'reserve-journal'
$stdout = $null
$stderr = $null
$errors = $null
$process = $null
$childStarted = $false

function Write-Receipt([string]$name, $value) {
    $path = Join-Path $journal $name
    $bytes = [Text.Encoding]::UTF8.GetBytes(($value | ConvertTo-Json -Depth 8) + "`n")
    $stream = [IO.File]::Open($path, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
    try { $stream.Write($bytes, 0, $bytes.Length); $stream.Flush($true) }
    finally { $stream.Dispose() }
}

try {
    # No -Force: an existing attempt is never reused, even after an early refusal.
    New-Item -ItemType Directory -Path $journal -ErrorAction Stop | Out-Null
    $stdout = [IO.FileStream]::new((Join-Path $journal 'stdout.log'), [IO.FileMode]::CreateNew,
        [IO.FileAccess]::Write, [IO.FileShare]::Read, 1, [IO.FileOptions]::WriteThrough)
    $stderr = [IO.FileStream]::new((Join-Path $journal 'stderr.log'), [IO.FileMode]::CreateNew,
        [IO.FileAccess]::Write, [IO.FileShare]::Read, 1, [IO.FileOptions]::WriteThrough)
    $errors = [IO.File]::Open((Join-Path $journal 'launcher-errors.log'), 'CreateNew', 'Write', 'Read')
    Write-Receipt 'launcher-intent.json' @{
        schema = 'hc074-preflight-journal-intent-v1'; retry = $false
        original_windows = @(300, 60, 10); previous_attempt = 'output_lost_execution_unconfirmed'
        root_lookup_authorized_by_this_receipt = $false
        admission = $false; server = $false; product = $false; original_owner_run = $false
        clock_change = $false; service_change = $false; tracing_state_changed = $false
    }
    $stage = 'resolve-python'
    # Windows PowerShell can return several Application matches; select one PATH result.
    $python = (Get-Command python.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
    $payload = Join-Path $PSScriptRoot 'root_namespace_logged.py'
    $stage = 'start-python'
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $python
    $info.Arguments = '-B -u "' + $payload + '" "' + $journal + '"'
    $info.WorkingDirectory = $root
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.EnvironmentVariables['PYTHONIOENCODING'] = 'utf-8'
    Write-Receipt 'launcher-command.json' @{ executable = $python; arguments = $info.Arguments; self_elevation = $false }
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $info
    $childStarted = $null
    if (-not $process.Start()) { $childStarted = $false; throw 'Python process did not start' }
    $childStarted = $true
    $stage = 'drain-python'
    Write-Receipt 'launcher-started.json' @{ child_started = $true; child_pid = $process.Id; admission = $false }
    # Drain both pipes concurrently: waiting on only one pipe can deadlock.
    $outCopy = $process.StandardOutput.BaseStream.CopyToAsync($stdout)
    $errCopy = $process.StandardError.BaseStream.CopyToAsync($stderr)
    $stage = 'wait-python'
    $process.WaitForExit()
    $outCopy.GetAwaiter().GetResult()
    $errCopy.GetAwaiter().GetResult()
    $stdout.Flush($true)
    $stderr.Flush($true)
    $exitCode = $process.ExitCode
    Write-Receipt 'launcher-result.json' @{ child_exit = $exitCode; stage = 'child-exited'; admission = $false; retry = $false }
} catch {
    $detail = ($_ | Out-String) + "`n" + $_.Exception.ToString() + "`n" + $_.ScriptStackTrace
    if ($null -ne $errors) {
        $bytes = [Text.Encoding]::UTF8.GetBytes($detail)
        $errors.Write($bytes, 0, $bytes.Length)
        $errors.Flush($true)
        Write-Receipt 'launcher-failure.json' @{ stage = $stage; child_started = $childStarted; error = $detail; retry = $false; admission = $false }
    }
    [Console]::Error.WriteLine($detail)
    $exitCode = 1
} finally {
    foreach ($stream in @($stdout, $stderr, $errors)) {
        if ($null -ne $stream) { $stream.Flush($true); $stream.Dispose() }
    }
    if ($null -ne $process) { $process.Dispose() }
}
[Console]::WriteLine('Diagnostic journal: ' + $journal + '; exit: ' + $exitCode + '; do not rerun.')
exit $exitCode
