#requires -Version 7.2
param(
    [Parameter(Mandatory = $true)][string]$SourceSha,
    [Parameter(Mandatory = $true)][string]$BinarySha256,
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory
)
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
Set-Location -LiteralPath $repo
if ($SourceSha -notmatch '^[a-f0-9]{40}$' -or $BinarySha256 -notmatch '^[a-f0-9]{64}$') { throw 'Full immutable source/binary identity required' }
if ((git rev-parse HEAD) -ne $SourceSha) { throw 'Source mismatch' }
if (git status --porcelain) { throw 'Execution seal requires a clean worktree' }
$binary = Join-Path $repo 'tools/get-owner-scheduled-controls-074/target/debug/memory-diagnostics-074.exe'
if ((Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant() -ne $BinarySha256) { throw 'Binary mismatch' }
$destination = [IO.Path]::GetFullPath((Join-Path $repo $EvidenceDirectory))
$allowed = [IO.Path]::GetFullPath((Join-Path $repo 'docs/testing/performance/0.74/local-runs')) + [IO.Path]::DirectorySeparatorChar
if (-not $destination.StartsWith($allowed, [StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $destination)) { throw 'Evidence must be a new directory under local-runs' }
[void](New-Item -ItemType Directory -Path $destination)
function Save-Json($name, $value) {
    $serialized = $value | ConvertTo-Json -Depth 30
    [IO.File]::WriteAllText((Join-Path $destination $name), $serialized + "`n", [Text.UTF8Encoding]::new($false))
}
$hashes = [ordered]@{}
foreach ($path in @('Cargo.lock', 'tools/get-owner-scheduled-controls-074/Cargo.lock',
    'tools/get-owner-scheduled-controls-074/src/bin/memory_diagnostics.rs',
    'tools/resp-scratch-screen-074/src/memory.rs',
    'docs/testing/performance/0.74/secure-observer-memory-checks-contract.toml',
    'docs/testing/performance/0.74/qualification-manifest.toml',
    'scripts/perf/performance_secure_memory_074.ps1')) {
    $hashes[$path] = (Get-FileHash -LiteralPath (Join-Path $repo $path) -Algorithm SHA256).Hash.ToLowerInvariant()
}
$surfaces = @('direct', 'hc1', 'hc2-mtls', 'resp2-mtls', 'resp3-mtls')
$payloads = @(256, 65536)
Save-Json 'seal.json' ([ordered]@{
    schema_version = 1; source_commit = $SourceSha; binary_sha256 = $BinarySha256
    binary_path = 'tools/get-owner-scheduled-controls-074/target/debug/memory-diagnostics-074.exe'
    build_command = 'CARGO_INCREMENTAL=0 cargo build --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --bin memory-diagnostics-074 --features allocation-diagnostics --locked'
    build_profile = 'dev-unoptimized-with-debug-info'; features = @('allocation-diagnostics')
    source_hashes = $hashes; os = [Environment]::OSVersion.VersionString
    surfaces = $surfaces; payload_bytes = $payloads; repeats = 3; maximum_attempts = 30
    process_timeout_seconds = 65; failed_attempts_allowed = 0; retry_allowed = $false
    created_at_utc = [DateTime]::UtcNow.ToString('o'); promotable = $false
    product_numeric_claims_allowed = $false; admission_allowed = $false
})
$attempts = [Collections.Generic.List[object]]::new()
$failed = $false
foreach ($repeat in 1..3) {
    foreach ($surface in $surfaces) {
        foreach ($payload in $payloads) {
            $id = '{0:D2}-{1}-{2}' -f $repeat, $surface, $payload
            $reason = $null
            $exitCode = $null
            $process = $null
            $started = $false
            try {
                if ((git rev-parse HEAD) -ne $SourceSha) { throw 'Source changed' }
                git diff --quiet
                if ($LASTEXITCODE -ne 0) { throw 'Tracked source changed' }
                git diff --cached --quiet
                if ($LASTEXITCODE -ne 0) { throw 'Index changed' }
                if ((Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash.ToLowerInvariant() -ne $BinarySha256) { throw 'Binary changed' }
                $info = [Diagnostics.ProcessStartInfo]::new()
                $info.FileName = $binary
                $info.Arguments = "$surface $payload"
                $info.UseShellExecute = $false
                $info.CreateNoWindow = $true
                $info.RedirectStandardOutput = $true
                $info.RedirectStandardError = $true
                $process = [Diagnostics.Process]::new()
                $process.StartInfo = $info
                if (-not $process.Start()) { throw 'Process did not start' }
                $started = $true
                $stdoutTask = $process.StandardOutput.ReadToEndAsync()
                $stderrTask = $process.StandardError.ReadToEndAsync()
                if (-not $process.WaitForExit(65000)) {
                    # Exact child created above only; never kill unrelated builds/services.
                    $process.Kill($true)
                    $process.WaitForExit()
                    $reason = 'External process deadline'
                }
                $stdout = $stdoutTask.GetAwaiter().GetResult()
                $stderr = $stderrTask.GetAwaiter().GetResult()
                [IO.File]::WriteAllText((Join-Path $destination "$id.stdout.json"), $stdout, [Text.UTF8Encoding]::new($false))
                [IO.File]::WriteAllText((Join-Path $destination "$id.stderr.txt"), $stderr, [Text.UTF8Encoding]::new($false))
                $exitCode = $process.ExitCode
                if ($reason) { throw $reason }
                if ($exitCode -ne 0) { throw "Diagnostic exit $exitCode" }
                $receipt = $stdout | ConvertFrom-Json
                if ($receipt.error -or $receipt.surface -ne $surface -or $receipt.payload_bytes -ne $payload -or
                    $receipt.seed -ne 740074 -or $receipt.keyspace -ne 16 -or $receipt.get_owner_feature -ne $false -or
                    $receipt.admission_allowed -ne $false -or $receipt.product_numeric_claims_allowed -ne $false -or
                    $receipt.cross_surface_numeric_comparison_allowed -ne $false -or
                    $receipt.allocator_active_resident_retained -ne $null -or
                    $receipt.allocator_retention_status -ne 'unavailable-not-a-pass' -or
                    $receipt.dataset_sha256 -notmatch '^[a-f0-9]{64}$' -or
                    (($receipt.phases.name -join ',') -ne 'preload,get,set,idle,delete,refill,shutdown')) { throw 'Diagnostic identity/phase/claim drift' }
                foreach ($phase in $receipt.phases) {
                    $empty = $phase.name -in @('delete', 'shutdown')
                    $expectedEntries = if ($empty) { 0 } else { 16 }
                    $expectedBytes = if ($empty) { 0 } else { 16 * $payload }
                    $expectedCalls = if ($phase.name -in @('get', 'set')) { 128 } elseif ($phase.name -in @('idle', 'shutdown')) { 0 } else { 16 }
                    if ($phase.logical_entries -ne $expectedEntries -or $phase.logical_value_bytes -ne $expectedBytes -or
                        $phase.workload_calls -ne $expectedCalls -or $phase.process_rss_bytes -le 0 -or
                        $phase.allocations.peak_live_requested_bytes -lt $phase.allocations.live_before_bytes) { throw 'Logical ownership or metric units drift' }
                }
            } catch { $reason = $_.Exception.Message; $failed = $true }
            finally {
                if ($process) {
                    if ($started -and -not $process.HasExited) { $process.Kill($true); $process.WaitForExit() }
                    $process.Dispose()
                }
            }
            $rawHash = if (Test-Path -LiteralPath (Join-Path $destination "$id.stdout.json")) {
                (Get-FileHash -LiteralPath (Join-Path $destination "$id.stdout.json") -Algorithm SHA256).Hash.ToLowerInvariant()
            } else { $null }
            $attempts.Add([ordered]@{ id = $id; surface = $surface; payload_bytes = $payload; repeat = $repeat; exit_code = $exitCode; error = $reason; stdout_sha256 = $rawHash })
            Save-Json 'summary.json' ([ordered]@{
                schema_version = 1; source_commit = $SourceSha; attempts = $attempts.ToArray()
                completed_attempts = $attempts.Count; expected_attempts = 30; stopped_on_failure = $failed
                diagnostic_cells_complete = (-not $failed -and $attempts.Count -eq 30)
                allocator_retention_admission = $false; product_numeric_claims_allowed = $false
                native_nonregression_measured = $false; full_d3_completed = $false; retry_allowed = $false
            })
            Write-Output "$id exit=$exitCode error=$reason"
            if ($failed) { exit 1 }
        }
    }
}
