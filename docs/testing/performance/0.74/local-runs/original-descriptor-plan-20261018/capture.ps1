param([Parameter(Mandatory)][ValidatePattern('^[a-f0-9]{40}$')][string]$SourceCommit)
$ErrorActionPreference='Stop'
$repo=(& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
$base='5a2cb676b58f7e17a2d2880ded2c56aae93b7a41'
$paths=@(& git -C $repo diff --name-only "$base..$SourceCommit")
if ($LASTEXITCODE -ne 0 -or $paths.Count -ne 10) { throw 'Implementation scope differs' }
$sourcePaths=@($paths)+@('Cargo.toml','Cargo.lock','tools/long-run-supervisor-074/Cargo.toml',
    'tools/long-run-supervisor-074/src/diagnostic_original_fence.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_collector_sinks.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_collector_pipes.rs',
    'tools/long-run-supervisor-074/src/diagnostic_original_grouped_preparation.rs',
    'tools/long-run-supervisor-074/src/diagnostic_autonomous_watchdog.rs',
    'tools/long-run-supervisor-074/src/diagnostic_kernel_admission.rs',
    'tools/long-run-supervisor-074/src/diagnostic_lease.rs')
$sources=foreach ($path in ($sourcePaths | Sort-Object -Unique)) {
    $blob=(& git -C $repo rev-parse "${SourceCommit}:$path").Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Committed source missing' }
    $current=(& git -C $repo hash-object "--path=$path" (Join-Path $repo $path)).Trim()
    if ($LASTEXITCODE -ne 0 -or $current -cne $blob) { throw 'Uncommitted source drift' }
    [ordered]@{ path=$path; blob=$blob; raw_sha256=(Get-FileHash -LiteralPath (Join-Path $repo $path)).Hash.ToLowerInvariant() }
}
$info=[Diagnostics.ProcessStartInfo]::new('git')
$info.UseShellExecute=$false; $info.CreateNoWindow=$true
$info.RedirectStandardOutput=$true; $info.RedirectStandardError=$true
foreach ($arg in @('-C',$repo,'diff','--binary',"$base..$SourceCommit")) { $info.ArgumentList.Add($arg) }
$process=[Diagnostics.Process]::Start($info)
$capsule=$process.StandardOutput.ReadToEnd(); $errors=$process.StandardError.ReadToEnd(); $process.WaitForExit()
if ($process.ExitCode -ne 0 -or $errors -ne '') { throw 'Capsule generation failed' }
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'implementation.patch'),$capsule,[Text.UTF8Encoding]::new($false))
$binaries=foreach ($line in (Get-Content (Join-Path $PSScriptRoot 'binary-hashes.log'))) {
    if ($line -cnotmatch '^([a-f0-9]{64})  tests-(194|188)$') { throw 'Invalid binary record' }
    [ordered]@{ compiler=$Matches[2]; sha256=$Matches[1] }
}
if (($binaries.compiler -join '|') -cne '194|188') { throw 'Binary inventory differs' }
$files=foreach ($file in (Get-ChildItem -LiteralPath $PSScriptRoot -File | Where-Object { $_.Name -cne 'manifest.json' } | Sort-Object Name)) {
    [ordered]@{ path=$file.Name; bytes=$file.Length; sha256=(Get-FileHash -LiteralPath $file.FullName).Hash.ToLowerInvariant() }
}
$manifest=[ordered]@{
    baseline_commit=$base; source_commit=$SourceCommit
    server_scratch='/tmp/hc074-original-descriptors-20261018.64pJ7n'
    initial_server_scratch='/tmp/hc074-original-descriptors-20261018.ltqPdB'
    initial_test_binary_194='4eb6133d1a5813a1ffb46fe092f79bb9c7a498f98a29531892f958e72e6dc410'
    initial_test_binary_188='4215a3f00bb89671652d83d186f32080b6b07450ac581e45611a0175e1f2149c'
    initial_188_executed=$false
    qualification_sha256='11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
    host_controls=6; screen_controls=1; host_refusals=20; host_holder_cases=24
    normal_eof=6; omitted_ack_eof=6; holder_sigkill_cuts=6; hidden_writer_refusals=6; hidden_cleanup_sigkills=6
    test_oracle_corrected=$true; offset_zero_required=$true
    guardian_enrollment=$false; product_load=$false; admission=$false
    binaries=@($binaries); sources=@($sources); files=@($files)
}
$json=($manifest | ConvertTo-Json -Depth 8).Replace("`r`n","`n")+"`n"
[IO.File]::WriteAllText((Join-Path $PSScriptRoot 'manifest.json'),$json,[Text.UTF8Encoding]::new($false))
