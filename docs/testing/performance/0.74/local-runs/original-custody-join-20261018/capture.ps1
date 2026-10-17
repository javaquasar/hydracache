param([Parameter(Mandatory)][ValidatePattern('^[a-f0-9]{40}$')][string]$SourceCommit)
$ErrorActionPreference = 'Stop'
$repo = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
$base = '97f7583d6dd61f953e73ce836f943ca125f89fe6'
$paths = @(& git -C $repo diff --name-only "$base..$SourceCommit")
if ($LASTEXITCODE -ne 0 -or $paths.Count -ne 14) { throw 'Implementation scope differs' }
$sourcePaths = @($paths) + @('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml',
    'tools/long-run-supervisor-074/src/diagnostic_original_fence.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes.rs',
    'tools/long-run-supervisor-074/src/diagnostic_collector_causal.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_lease.rs',
    'tools/long-run-supervisor-074/src/diagnostic_autonomous_watchdog.rs',
    'tools/long-run-supervisor-074/src/watchdog.rs')
$sources = foreach ($path in ($sourcePaths | Sort-Object -Unique)) {
    $blob = (& git -C $repo rev-parse "${SourceCommit}:$path").Trim()
    if ($LASTEXITCODE -ne 0) { throw "Source missing: $path" }
    $current = (& git -C $repo hash-object "--path=$path" (Join-Path $repo $path)).Trim()
    if ($LASTEXITCODE -ne 0 -or $current -cne $blob) { throw "Uncommitted source drift: $path" }
    [ordered]@{ path=$path; blob=$blob; raw_sha256=(Get-FileHash -LiteralPath (Join-Path $repo $path)).Hash.ToLowerInvariant() }
}
$info = [Diagnostics.ProcessStartInfo]::new('git')
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
foreach ($arg in @('-C',$repo,'diff','--binary',"$base..$SourceCommit")) { $info.ArgumentList.Add($arg) }
$process = [Diagnostics.Process]::Start($info)
$capsule = $process.StandardOutput.ReadToEnd()
$errors = $process.StandardError.ReadToEnd()
$process.WaitForExit()
if ($process.ExitCode -ne 0 -or $errors -ne '') { throw 'Capsule generation failed' }
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'implementation.patch'), $capsule, [Text.UTF8Encoding]::new($false))
$files = foreach ($file in (Get-ChildItem -LiteralPath $PSScriptRoot -File | Where-Object { $_.Name -cne 'manifest.json' } | Sort-Object Name)) {
    [ordered]@{ path=$file.Name; bytes=$file.Length; sha256=(Get-FileHash -LiteralPath $file.FullName).Hash.ToLowerInvariant() }
}
$manifest = [ordered]@{
    baseline_commit=$base; source_commit=$SourceCommit
    rejected_base_commit='725f9677512a2249b1952962826d8032a764a4cf'
    rejected_source='double-frame-source.patch'; rejected_admission=$false
    rejected_server_scratch='/tmp/hc074-custody-join-20261018.ZrDR8A'
    server_scratch='/tmp/hc074-custody-join-20261018.PvmgFS'
    qualification_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
    host_controls=6; screen_controls=1; host_refusals=40
    host_process_cases=30; host_sigkills=18; host_normal_returns=6; host_lost_acks=6; host_fresh_inspections=60
    guardian_enrollment=$false; product_load=$false; admission=$false
    binaries=@(
        [ordered]@{ compiler='194'; sha256='e2f801a5004e0a3c63b093e5ba73f628c4cbad2224fa8ecde377b57822f7b64c' }
        [ordered]@{ compiler='188'; sha256='e6b4d29057079fbf8d294affb42245f1e37abf28c41a634ca84f97e57eb00c7d' }
    )
    rejected_binaries=@(
        [ordered]@{ compiler='194'; sha256='6322f4039d6721bc1a6db3352f29f46aac974ffba71c7db20f8a87a039081312' }
        [ordered]@{ compiler='188'; sha256='74bc38fbe752a1465e4895155605cee37fc25af7fe69a84fe9586183b23fb662' }
    )
    sources=@($sources); files=@($files)
}
$json = ($manifest | ConvertTo-Json -Depth 8).Replace("`r`n", "`n") + "`n"
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'manifest.json'), $json, [Text.UTF8Encoding]::new($false))
