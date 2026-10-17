param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$cases = @('normal','controller-kill','controller-stop','hidden-writer','stopped-writer','ack-omitted','ack-disconnected','write-failure','sync-failure','size-limit','overflow','collector-stop')
function Check-Rows([string]$raw) {
    $lane = ''
    $index = 0
    $summaries = 0
    $done = @()
    foreach ($line in ($raw -split "\r?\n")) {
        if ($line.Contains('HC074-AUTONOMOUS-COHORT')) {
            if ($line -cmatch '^HC074-AUTONOMOUS-COHORT compiler=(194|188)$') {
                if ($lane -ne '' -or $done -contains $Matches[1]) { throw 'Duplicate/interleaved cohort' }
                $lane = $Matches[1]; $index = 0; $summaries = 0
            } elseif ($line -cmatch '^HC074-AUTONOMOUS-COHORT compiler=(194|188) completed=true$') {
                if ($lane -ne $Matches[1] -or $index -ne 36 -or $summaries -ne 1) { throw 'Incomplete cohort' }
                $done += $lane; $lane = ''
            } else { throw 'Malformed cohort' }
        } elseif ($line.Contains('HC074-AUTONOMOUS case=')) {
            if ($line -cnotmatch '^(?:test watchdog::autonomous::tests::autonomous_writer_fence_matrix \.\.\. )?HC074-AUTONOMOUS case=([a-z-]+) repeat=([1-3]) seed=0x7407a17 elapsed_ms=([0-9]+) completed=true$') { throw 'Malformed case' }
            $case = $Matches[1]; $repeat = [int]$Matches[2]
            if ($lane -eq '' -or $index -ge 36 -or $summaries -ne 0 -or $case -cne $cases[$index % 12] -or $repeat -ne (1 + [math]::Floor($index / 12))) { throw 'Missing/duplicate/reordered case' }
            $index++
        } elseif ($line.Contains('HC074-AUTONOMOUS cases=')) {
            if ($line -cne 'HC074-AUTONOMOUS cases=36 repeats=3 completed=true' -or $lane -eq '' -or $index -ne 36) { throw 'Bad summary' }
            $summaries++
        }
    }
    if ($lane -ne '' -or ($done -join ',') -cne '194,188') { throw 'Cohort set differs' }
    if ($raw -cmatch 'FAILED|panicked|error:|timed out|unexpected_executable=') { throw 'Failure in raw log' }
}
if ($SelfTest) {
    $valid = ''
    $nl = [string][char]10
    foreach ($lane in @('194','188')) {
        $valid += "HC074-AUTONOMOUS-COHORT compiler=$lane" + $nl
        foreach ($repeat in 1..3) { foreach ($case in $cases) {
            $valid += "HC074-AUTONOMOUS case=$case repeat=$repeat seed=0x7407a17 elapsed_ms=1 completed=true" + $nl
        } }
        $valid += 'HC074-AUTONOMOUS cases=36 repeats=3 completed=true' + $nl + "HC074-AUTONOMOUS-COHORT compiler=$lane completed=true" + $nl
    }
    $valid += 'test result: ok. 4 passed; 0 failed;' + $nl
    Check-Rows $valid
    $mutants = @(
        $valid.Replace('seed=0x7407a17','seed=0x7407a18'),
        $valid.Replace('case=hidden-writer','case=unknown'),
        $valid.Replace('case=normal repeat=1','case=normal repeat=2'),
        $valid.Replace('completed=true','completed=false'),
        $valid.Replace(('HC074-AUTONOMOUS case=normal repeat=1 seed=0x7407a17 elapsed_ms=1 completed=true' + $nl),''),
        $valid.Replace('cases=36','cases=35'),
        $valid.Replace('compiler=188','compiler=194'),
        ($valid + 'error: trailing failure' + $nl),
        ($valid + 'test result: FAILED. 0 passed; 1 failed;' + $nl),
        $valid.Replace('case=overflow repeat=3','case=normal repeat=1')
    )
    foreach ($mutant in $mutants) {
        $refused = $false
        try { Check-Rows $mutant } catch { $refused = $true }
        if (-not $refused) { throw 'Verifier accepted mutation' }
    }
    Write-Output 'HC074-AUTONOMOUS-VERIFIER selftest=true accepted=1 refused=10'
    return
}
$manifest = Get-Content -Raw (Join-Path $PSScriptRoot 'manifest.json') | ConvertFrom-Json
$names = @(Get-ChildItem -LiteralPath $PSScriptRoot -File | ForEach-Object { $_.Name } | Sort-Object)
$declared = @(@($manifest.files | ForEach-Object { $_.path }) + @('manifest.json') | Sort-Object)
if (($names -join '|') -cne ($declared -join '|')) { throw 'Evidence inventory differs' }
foreach ($file in $manifest.files) {
    $path = Join-Path $PSScriptRoot $file.path
    if ((Get-Item -LiteralPath $path).Length -ne $file.bytes -or
        (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $file.sha256) { throw "Evidence drift: $($file.path)" }
}
$raw = Get-Content -Raw (Join-Path $PSScriptRoot 'host.log')
Check-Rows $raw
$local = Get-Content -Raw (Join-Path $PSScriptRoot 'wsl-debug.log')
if ([regex]::Matches($local,'HC074-AUTONOMOUS case=').Count -ne 36 -or
    -not $local.Contains('HC074-AUTONOMOUS cases=36 repeats=3 completed=true') -or
    -not $local.Contains('test result: ok. 1 passed; 0 failed;') -or $local -cmatch 'FAILED|panicked|error:') { throw 'Local matrix incomplete' }
foreach ($binary in $manifest.binaries) {
    if (-not $raw.Contains("$($binary.sha256)  tests-$($binary.compiler)")) { throw 'Binary digest absent from host log' }
}
foreach ($script in @('guard.sh','run.sh')) {
    $file = $manifest.files | Where-Object { $_.path -ceq $script }
    if (-not $raw.Contains("$($file.sha256)  $script")) { throw 'Executed script digest differs' }
}
if ([regex]::Matches($raw,'preflight=ok supervisor_pid=8839 restarts=0').Count -ne 5 -or
    [regex]::Matches($raw,'test result: ok\. 4 passed; 0 failed;').Count -ne 2 -or
    [regex]::Matches($raw,'test result: ok\. 1 passed; 0 failed;').Count -ne 2 -or
    -not $raw.TrimEnd().EndsWith('HC074-AUTONOMOUS-HOST completed=true supervisor_unchanged=true no_fixture_executable=true')) { throw 'Host envelope incomplete' }
$repo = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
foreach ($source in $manifest.sources) {
    $blob = (& git -C $repo rev-parse "$($manifest.source_commit):$($source.path)").Trim()
    if ($LASTEXITCODE -ne 0 -or $blob -cne $source.blob) { throw 'Committed source drift' }
    $current = (& git -C $repo hash-object "--path=$($source.path)" (Join-Path $repo $source.path)).Trim()
    if ($LASTEXITCODE -ne 0 -or $current -cne $blob) { throw 'Working source drift' }
}
function Check-Capsule([string]$commit, [string]$name) {
$info = [Diagnostics.ProcessStartInfo]::new('git')
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
foreach ($arg in @('-C',$repo,'diff','--binary',"$($manifest.baseline_commit)..$commit")) { $info.ArgumentList.Add($arg) }
$process = [Diagnostics.Process]::Start($info)
$expected = $process.StandardOutput.ReadToEnd()
$errors = $process.StandardError.ReadToEnd()
$process.WaitForExit()
if ($process.ExitCode -ne 0 -or $errors -ne '' -or
    $expected -cne [IO.File]::ReadAllText((Join-Path $PSScriptRoot $name))) { throw 'Source capsule differs from exact committed diff' }
}
Check-Capsule $manifest.source_commit 'implementation.patch'
Check-Capsule $manifest.initial_cohort.source_commit $manifest.initial_cohort.source_patch
$initial = Get-Content -Raw (Join-Path $PSScriptRoot $manifest.initial_cohort.host_log)
Check-Rows $initial
foreach ($binary in $manifest.initial_cohort.binaries) {
    if (-not $initial.Contains("$($binary.sha256)  tests-$($binary.compiler)")) { throw 'Initial binary cohort differs' }
}
& git -C $repo apply --reverse --check (Join-Path $PSScriptRoot 'implementation.patch')
if ($LASTEXITCODE -ne 0) { throw 'Source capsule cannot reverse current implementation' }
$qualification = Join-Path $repo 'docs/testing/performance/0.74/qualification-manifest.toml'
if ((Get-FileHash -LiteralPath $qualification -Algorithm SHA256).Hash.ToLowerInvariant() -cne $manifest.qualification_sha256) { throw 'Qualification changed' }
Write-Output 'HC074-AUTONOMOUS-EVIDENCE verified=true cases=72 production_join=false admission=false'
