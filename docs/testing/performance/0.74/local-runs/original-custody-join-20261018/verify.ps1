param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
function Expected-Events {
    foreach ($sample in 0..2) { "HC074-CUSTODY-JOIN-CONTROL sample=$sample bind_ipc=34 validate_ipc=34 original_origins=true unchanged=true eof=true send=false admission=false" }
    foreach ($case in @('trust/opened','trust/retained','bundle-content/retained','output/retained','message/retained','writer/retained','projection/retained','state/retained','alarm/retained','custody-missing/retained','custody-object/retained','custody-flags/retained','expiry/retained')) {
        $name, $cut = $case.Split('/')
        "HC074-CUSTODY-JOIN-REFUSAL seed=0x0740c518 case=$name cut=$cut sticky=true held=true unchanged=true send=false"
    }
    foreach ($case in @('trust','custody-object','custody-flags','custody-missing','fixture-origin','final-expiry','production-bind')) { "HC074-CUSTODY-JOIN-LATER case=$case sticky=true held=true unchanged=true send=false" }
    foreach ($repeat in 1..3) {
        # Exact xorshift permutation for the preregistered 0x0740c618 seed.
        foreach ($pair in @('returned-unacknowledged/kill','retained/kill','opened/kill','returned-unacknowledged/control','returned-unacknowledged/ack-lost')) {
            $stage, $mode = $pair.Split('/')
            foreach ($inspection in 1..2) { "HC074-CUSTODY-JOIN-INSPECTION repeat=$repeat stage=$stage mode=$mode inspection=$inspection refused=true unchanged=true" }
            $kill = if ($mode -ceq 'kill') { 'true' } else { 'false' }
            $acks = if ($mode -ceq 'control') { 1 } else { 0 }
            "HC074-CUSTODY-JOIN-PROCESS seed=0x0740c618 repeat=$repeat stage=$stage mode=$mode sigkill=$kill acks=$acks held=true unchanged=true send=false"
        }
        "HC074-CUSTODY-JOIN-PROCESS-SUMMARY seed=0x0740c618 repeat=$repeat sigkills=3 controls=1 lost_acks=1 inspections=10 admission=false"
    }
}
function Check-Events([string]$raw) {
    $actual = @()
    foreach ($line in ($raw -split '\r?\n')) {
        if ($line -cmatch 'HC074-CUSTODY-JOIN-(CONTROL|REFUSAL|LATER|INSPECTION|PROCESS(?:-SUMMARY)?) ') {
            $actual += [regex]::Replace($line, '^test [A-Za-z0-9_:]+ \.\.\. ', '')
        }
    }
    if (($actual -join "`n") -cne ((Expected-Events) -join "`n") -or
        $raw -cmatch 'FAILED|panicked|error:|timed out|unexpected_executable=') { throw 'Missing, duplicated, reordered or invalid event' }
}
function Check-Host([string]$raw) {
    if ([regex]::Matches($raw, '(?m)^HC074-CUSTODY-JOIN-COHORT.*$').Count -ne 4) { throw 'Invalid cohort envelope' }
    $last = -1
    foreach ($compiler in @('194','188')) {
        $blocks = [regex]::Matches($raw, "(?ms)^HC074-CUSTODY-JOIN-COHORT compiler=$compiler\r?\n(.*?)^HC074-CUSTODY-JOIN-COHORT compiler=$compiler completed=true\r?\n")
        if ($blocks.Count -ne 1 -or $blocks[0].Index -le $last) { throw 'Cohort order differs' }
        $last = $blocks[0].Index
        $body = $blocks[0].Groups[1].Value
        Check-Events $body
        $phases = @([regex]::Matches($body, '(?m)^HC074-CUSTODY-JOIN-PHASE name=([a-z]+) (begin|completed)=true\r?$') | ForEach-Object { "$($_.Groups[1].Value)/$($_.Groups[2].Value)" })
        if (($phases -join '|') -cne 'healthy/begin|healthy/completed|refusals/begin|refusals/completed|later/begin|later/completed|process/begin|process/completed') { throw 'Phase order differs' }
        foreach ($phase in @('healthy','refusals','later','process')) {
            $part = [regex]::Matches($body, "(?ms)^HC074-CUSTODY-JOIN-PHASE name=$phase begin=true\r?\n(.*?)^HC074-CUSTODY-JOIN-PHASE name=$phase completed=true\r?\n")
            if ($part.Count -ne 1 -or [regex]::Matches($part[0].Groups[1].Value, 'test result: ok\. 1 passed; 0 failed;').Count -ne 1) { throw 'Phase did not pass exactly once' }
        }
        foreach ($count in @(32,3)) {
            if ([regex]::Matches($body, "test result: ok\. $count passed; 0 failed;").Count -ne 1) { throw 'Fast subtree missing' }
        }
    }
    if ([regex]::Matches($raw, 'preflight=ok supervisor_pid=8839 restarts=0').Count -ne 5 -or
        -not $raw.TrimEnd().EndsWith('HC074-CUSTODY-JOIN-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false') -or
        $raw -cmatch 'FAILED|panicked|error:|timed out|unexpected_executable=') { throw 'Host postflight incomplete' }
}
if ($SelfTest) {
    $rows = @(Expected-Events)
    $good = $rows -join "`n"
    Check-Events ($good + "`ntest result: ok. 1 passed; 0 failed;")
    $mutations = @(
        ($good.Replace($rows[0] + "`n", '')), ($good + "`n" + $rows[0]),
        ($good.Replace($rows[0], $rows[1])), ($good.Replace('0x0740c618','0x0740c619')),
        ($good.Replace('0x0740c518','0x0740c519')), ($good.Replace('sigkill=true','sigkill=false')),
        ($good.Replace('acks=0','acks=1')), ($good.Replace('inspection=2','inspection=1')),
        ($good.Replace('bind_ipc=34','bind_ipc=68')), ($good.Replace('sticky=true','sticky=false')),
        ($good.Replace($rows[-1],'')), ($good + "`nFAILED"), ($good + "`npanicked at fixture")
    )
    foreach ($mutation in $mutations) {
        $refused = $false
        try { Check-Events $mutation } catch { $refused = $true }
        if (-not $refused) { throw 'Negative self-test accepted' }
    }
    $guard = 'HC074-CUSTODY-JOIN-SERVER preflight=ok supervisor_pid=8839 restarts=0'
    $hostRows = @($guard)
    foreach ($compiler in @('194','188')) {
        $hostRows += @($guard, "HC074-CUSTODY-JOIN-COHORT compiler=$compiler", 'test result: ok. 32 passed; 0 failed;', 'test result: ok. 3 passed; 0 failed;')
        $offset = 0
        foreach ($phase in @(@('healthy',3),@('refusals',13),@('later',7),@('process',48))) {
            $phaseName, $length = $phase
            $hostRows += "HC074-CUSTODY-JOIN-PHASE name=$phaseName begin=true"
            $hostRows += $rows[$offset..($offset + $length - 1)]
            $offset += $length
            $hostRows += @('test result: ok. 1 passed; 0 failed;', "HC074-CUSTODY-JOIN-PHASE name=$phaseName completed=true")
        }
        $hostRows += @("HC074-CUSTODY-JOIN-COHORT compiler=$compiler completed=true", $guard)
    }
    $hostRows += 'HC074-CUSTODY-JOIN-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false'
    $goodHost = ($hostRows -join "`n") + "`n"
    Check-Host $goodHost
    foreach ($mutation in @(
        ($goodHost.Replace($guard + "`n",'')),
        ($goodHost.Replace('name=healthy begin=true','name=refusals begin=true')),
        ($goodHost.Replace('test result: ok. 32 passed; 0 failed;', '')),
        ($goodHost.Replace('test result: ok. 1 passed; 0 failed;', 'test result: ok. 1 passed; 1 failed;')),
        ($goodHost.Replace('compiler=188','compiler=194'))
    )) {
        $refused = $false
        try { Check-Host $mutation } catch { $refused = $true }
        if (-not $refused) { throw 'Host negative self-test accepted' }
    }
    Write-Output 'HC074-CUSTODY-JOIN-VERIFIER selftest=true positive=2 negative=18'
    return
}
$manifest = Get-Content -Raw (Join-Path $PSScriptRoot 'manifest.json') | ConvertFrom-Json
if ($manifest.admission -cne $false -or $manifest.product_load -cne $false -or
    $manifest.guardian_enrollment -cne $false -or $manifest.rejected_admission -cne $false -or
    $manifest.host_controls -ne 6 -or $manifest.screen_controls -ne 1 -or
    $manifest.host_refusals -ne 40 -or $manifest.host_process_cases -ne 30 -or
    $manifest.host_sigkills -ne 18 -or $manifest.host_normal_returns -ne 6 -or
    $manifest.host_lost_acks -ne 6 -or $manifest.host_fresh_inspections -ne 60) { throw 'Scope or derived counts differ' }
$names = @(Get-ChildItem -LiteralPath $PSScriptRoot -File | ForEach-Object { $_.Name } | Sort-Object)
$declared = @(@($manifest.files | ForEach-Object { $_.path }) + @('manifest.json') | Sort-Object)
if (($names -join '|') -cne ($declared -join '|')) { throw 'Evidence inventory differs' }
foreach ($file in $manifest.files) {
    if ($file.path -cnotmatch '^[A-Za-z0-9_.-]+$') { throw 'Evidence path refused' }
    $path = Join-Path $PSScriptRoot $file.path
    if ((Get-Item -LiteralPath $path).Length -ne $file.bytes -or
        (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -cne $file.sha256) { throw "Evidence drift: $($file.path)" }
}
foreach ($gate in @(
    @('static.log','test result: ok. 204 passed; 0 failed;'),
    @('doc-check.log','doc-check: OK (releases.toml is consistent)'),
    @('registry.log','gated-test-check: OK (184 discovered gates, all registered)'),
    @('local-contract.log','performance-contract-check 0.74: OK (local, non-promotable)')
)) {
    $log = Get-Content -Raw (Join-Path $PSScriptRoot $gate[0])
    if (-not $log.Contains($gate[1]) -or $log -cmatch 'FAILED|panicked|error:') { throw "Gate incomplete: $($gate[0])" }
}
foreach ($compiler in @('194','188')) {
    $build = Get-Content -Raw (Join-Path $PSScriptRoot "$compiler-build.log")
    if (-not $build.Contains('test result: ok. 8 passed; 0 failed;') -or
        [regex]::Matches($build,'Finished `dev`').Count -ne 2 -or
        [regex]::Matches($build,'Finished `release`').Count -ne 1 -or
        $build -cmatch 'FAILED|panicked|error:|warning:') { throw 'Compiler gate incomplete' }
}
$raw = Get-Content -Raw (Join-Path $PSScriptRoot 'host.log')
Check-Host $raw
$screen = Get-Content -Raw (Join-Path $PSScriptRoot 'screen.log')
if ([regex]::Matches($screen, [regex]::Escape(@(Expected-Events)[0])).Count -ne 1 -or
    -not $screen.Contains('test result: ok. 1 passed; 0 failed;') -or
    -not $screen.TrimEnd().EndsWith('HC074-CUSTODY-JOIN-SCREEN completed=true admission=false') -or
    $screen -cmatch 'FAILED|panicked|error:|timed out') { throw 'Healthy screen incomplete' }
foreach ($binary in $manifest.binaries) {
    if (-not $raw.Contains("$($binary.sha256)  tests-$($binary.compiler)")) { throw 'Host binary digest differs' }
}
foreach ($script in @('guard.sh','screen.sh','run.sh')) {
    $file = $manifest.files | Where-Object { $_.path -ceq $script }
    if (-not $raw.Contains("$($file.sha256)  $script")) { throw 'Executed script differs' }
}
foreach ($compiler in @('194','188')) {
    $negative = Get-Content -Raw (Join-Path $PSScriptRoot "double-frame-$compiler-screen.log")
    $binary = $manifest.rejected_binaries | Where-Object { $_.compiler -ceq $compiler }
    if (-not $negative.Contains('Causal(Time)') -or -not $negative.Contains('test result: FAILED. 0 passed; 1 failed;') -or
        -not $negative.Contains("$($binary.sha256)  tests-$compiler") -or
        $negative.Contains('HC074-CUSTODY-JOIN-SCREEN completed=true')) { throw 'Rejected screen missing or misclassified' }
}
$baseline = Get-Content -Raw (Join-Path $PSScriptRoot 'baseline.log')
if ([regex]::Matches($baseline, 'HC074-FIXED-GROUPED-CONTROL sample=0 original=true unchanged=true checkpoints=5 bounds_original=true reference=true guardian=false send=false admission=false').Count -ne 2 -or
    [regex]::Matches($baseline, 'test result: ok\. 1 passed; 0 failed;').Count -ne 2 -or
    [regex]::Matches($baseline, 'authentication_calls: 34,').Count -ne 4 -or
    $baseline -cmatch 'FAILED|panicked|error:|timed out') { throw 'Unchanged baseline controls incomplete' }
$repo = (& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
foreach ($source in $manifest.sources) {
    $blob = (& git -C $repo rev-parse "$($manifest.source_commit):$($source.path)").Trim()
    if ($LASTEXITCODE -ne 0 -or $blob -cne $source.blob) { throw 'Committed source drift' }
    $current = (& git -C $repo hash-object "--path=$($source.path)" (Join-Path $repo $source.path)).Trim()
    if ($LASTEXITCODE -ne 0 -or $current -cne $blob) { throw 'Working source drift' }
}
$buildInputs = @(Get-Content (Join-Path $PSScriptRoot 'source-hashes.log'))
if ($buildInputs.Count -ne 13) { throw 'Build source inventory differs' }
foreach ($line in $buildInputs) {
    if ($line -cnotmatch '^([a-f0-9]{64})  ([A-Za-z0-9_./-]+)$') { throw 'Invalid build source record' }
    $hash, $path = $Matches[1], $Matches[2]
    $source = @($manifest.sources | Where-Object { $_.path -ceq $path })
    if ($source.Count -ne 1 -or $source[0].raw_sha256 -cne $hash) { throw 'Saved build input differs' }
}
$info = [Diagnostics.ProcessStartInfo]::new('git')
$info.UseShellExecute = $false
$info.CreateNoWindow = $true
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
foreach ($arg in @('-C',$repo,'diff','--binary',"$($manifest.baseline_commit)..$($manifest.source_commit)")) { $info.ArgumentList.Add($arg) }
$process = [Diagnostics.Process]::Start($info)
$expected = $process.StandardOutput.ReadToEnd()
$errors = $process.StandardError.ReadToEnd()
$process.WaitForExit()
if ($process.ExitCode -ne 0 -or $errors -ne '' -or
    $expected -cne [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'implementation.patch'))) { throw 'Committed source capsule differs' }
& git -C $repo apply --reverse --check (Join-Path $PSScriptRoot 'implementation.patch')
if ($LASTEXITCODE -ne 0) { throw 'Source capsule cannot reverse current implementation' }
$qualification = Join-Path $repo 'docs/testing/performance/0.74/qualification-manifest.toml'
if ((Get-FileHash -LiteralPath $qualification -Algorithm SHA256).Hash.ToLowerInvariant() -cne $manifest.qualification_sha256) { throw 'Qualification changed' }
Write-Output 'HC074-CUSTODY-JOIN-EVIDENCE verified=true host_controls=6 refusals=40 process_cases=30 fresh_inspections=60 admission=false'
