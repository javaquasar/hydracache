param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
function Expected-Events {
    foreach ($sample in 0..2) { "HC074-ORIGINAL-DESCRIPTORS-CONTROL sample=$sample plan_ipc=34 validate_ipc=34 guardian=6 collector=4 excluded_writers=4 unchanged=true send=false" }
    foreach ($case in @('trust','output','writer','custody-flags','message','alarm','preexisting-offset')) { "HC074-ORIGINAL-DESCRIPTORS-REFUSAL seed=0x0740d018 case=$case sticky=true held=true unchanged=true send=false" }
    foreach ($case in @('trust','custody-object','final-expiry')) { "HC074-ORIGINAL-DESCRIPTORS-LATER case=$case sticky=true held=true unchanged=true send=false" }
    foreach ($repeat in 1..3) {
        foreach ($case in @('normal','ack-lost','holder-kill','hidden-writer')) {
            $eof = if ($case -cin @('normal','ack-lost')) { 'true' } else { 'false' }
            "HC074-ORIGINAL-DESCRIPTOR-HOLDER seed=0x0740d118 repeat=$repeat case=$case eof=$eof retained=true unchanged=true send=false"
        }
    }
    'HC074-ORIGINAL-DESCRIPTOR-HOLDER completed=true cases=12 product=false'
}
function Check-Events([string]$raw) {
    $actual = @()
    foreach ($line in ($raw -split '\r?\n')) {
        if ($line -cmatch 'HC074-ORIGINAL-DESCRIPTORS-(CONTROL|REFUSAL|LATER) |HC074-ORIGINAL-DESCRIPTOR-HOLDER ') {
            $actual += [regex]::Replace($line, '^test [A-Za-z0-9_:]+ \.\.\. ', '')
        }
    }
    if (($actual -join "`n") -cne ((Expected-Events) -join "`n") -or
        $raw -cmatch 'FAILED|panicked|error:|timed out|unexpected_executable=') { throw 'Invalid, missing, duplicate or reordered event' }
}
function Check-Host([string]$raw) {
    if ([regex]::Matches($raw, '(?m)^HC074-ORIGINAL-DESCRIPTORS-COHORT.*$').Count -ne 4) { throw 'Cohort envelope differs' }
    $last = -1
    foreach ($compiler in @('194','188')) {
        $blocks = [regex]::Matches($raw, "(?ms)^HC074-ORIGINAL-DESCRIPTORS-COHORT compiler=$compiler\r?\n(.*?)^HC074-ORIGINAL-DESCRIPTORS-COHORT compiler=$compiler completed=true\r?\n")
        if ($blocks.Count -ne 1 -or $blocks[0].Index -le $last) { throw 'Cohort missing, duplicated or reordered' }
        $last = $blocks[0].Index
        $body = $blocks[0].Groups[1].Value
        Check-Events $body
        $phases = @([regex]::Matches($body, '(?m)^HC074-ORIGINAL-DESCRIPTORS-PHASE name=([a-z]+) (begin|completed)=true\r?$') | ForEach-Object { "$($_.Groups[1].Value)/$($_.Groups[2].Value)" })
        if (($phases -join '|') -cne 'healthy/begin|healthy/completed|refusals/begin|refusals/completed|later/begin|later/completed|holder/begin|holder/completed') { throw 'Phase order differs' }
        foreach ($phase in @('healthy','refusals','later','holder')) {
            $part = [regex]::Matches($body, "(?ms)^HC074-ORIGINAL-DESCRIPTORS-PHASE name=$phase begin=true\r?\n(.*?)^HC074-ORIGINAL-DESCRIPTORS-PHASE name=$phase completed=true\r?\n")
            if ($part.Count -ne 1 -or [regex]::Matches($part[0].Groups[1].Value, 'test result: ok\. 1 passed; 0 failed;').Count -ne 1) { throw 'Phase incomplete' }
        }
        foreach ($count in @(32,4)) { if ([regex]::Matches($body,"test result: ok\. $count passed; 0 failed;").Count -ne 1) { throw 'Fast subtree incomplete' } }
    }
    if ([regex]::Matches($raw,'preflight=ok supervisor_pid=8839 restarts=0').Count -ne 5 -or
        -not $raw.TrimEnd().EndsWith('HC074-ORIGINAL-DESCRIPTORS-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false') -or
        $raw -cmatch 'FAILED|panicked|error:|timed out|unexpected_executable=') { throw 'Host postflight incomplete' }
}
if ($SelfTest) {
    $rows = @(Expected-Events)
    $good = ($rows -join "`n") + "`ntest result: ok. 1 passed; 0 failed;"
    Check-Events $good
    foreach ($mutation in @(
        ($good.Replace($rows[0]+"`n",'')), ($good+"`n"+$rows[0]), ($good.Replace($rows[0],$rows[1])),
        ($good.Replace('0x0740d118','0x0740d119')), ($good.Replace('0x0740d018','0x0740d019')),
        ($good.Replace('case=ack-lost','case=normal')), ($good.Replace('case=hidden-writer eof=false','case=hidden-writer eof=true')),
        ($good.Replace('excluded_writers=4','excluded_writers=2')), ($good.Replace('plan_ipc=34','plan_ipc=17')),
        ($good.Replace('sticky=true','sticky=false')), ($good.Replace($rows[-1],'')),
        ($good+"`nFAILED"), ($good+"`npanicked at fixture")
    )) {
        $refused = $false
        try { Check-Events $mutation } catch { $refused = $true }
        if (-not $refused) { throw 'Negative event self-test accepted' }
    }
    $guard = 'HC074-ORIGINAL-DESCRIPTORS-SERVER preflight=ok supervisor_pid=8839 restarts=0'
    $hostRows = @($guard)
    foreach ($compiler in @('194','188')) {
        $hostRows += @($guard,"HC074-ORIGINAL-DESCRIPTORS-COHORT compiler=$compiler",'test result: ok. 32 passed; 0 failed;','test result: ok. 4 passed; 0 failed;')
        $offset = 0
        foreach ($phase in @(@('healthy',3),@('refusals',7),@('later',3),@('holder',13))) {
            $name,$length = $phase
            $hostRows += "HC074-ORIGINAL-DESCRIPTORS-PHASE name=$name begin=true"
            $hostRows += $rows[$offset..($offset+$length-1)]
            $offset += $length
            $hostRows += @('test result: ok. 1 passed; 0 failed;',"HC074-ORIGINAL-DESCRIPTORS-PHASE name=$name completed=true")
        }
        $hostRows += @("HC074-ORIGINAL-DESCRIPTORS-COHORT compiler=$compiler completed=true",$guard)
    }
    $hostRows += 'HC074-ORIGINAL-DESCRIPTORS-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false'
    $goodHost = ($hostRows -join "`n")+"`n"
    Check-Host $goodHost
    foreach ($mutation in @(
        ($goodHost.Replace($guard+"`n",'')), ($goodHost.Replace('name=healthy begin=true','name=holder begin=true')),
        ($goodHost.Replace('test result: ok. 32 passed; 0 failed;','')),
        ($goodHost.Replace('test result: ok. 1 passed; 0 failed;','test result: ok. 1 passed; 1 failed;')),
        ($goodHost.Replace('compiler=188','compiler=194'))
    )) {
        $refused=$false
        try { Check-Host $mutation } catch { $refused=$true }
        if (-not $refused) { throw 'Negative host self-test accepted' }
    }
    Write-Output 'HC074-ORIGINAL-DESCRIPTORS-VERIFIER selftest=true positive=2 negative=18'
    return
}
$manifest=Get-Content -Raw (Join-Path $PSScriptRoot 'manifest.json') | ConvertFrom-Json
if ($manifest.admission -cne $false -or $manifest.guardian_enrollment -cne $false -or $manifest.product_load -cne $false -or
    $manifest.host_controls -ne 6 -or $manifest.host_refusals -ne 20 -or $manifest.host_holder_cases -ne 24) { throw 'Scope or derived counts differ' }
$names=@(Get-ChildItem -LiteralPath $PSScriptRoot -File | ForEach-Object Name | Sort-Object)
$declared=@(@($manifest.files | ForEach-Object path)+@('manifest.json') | Sort-Object)
if (($names -join '|') -cne ($declared -join '|')) { throw 'Evidence inventory differs' }
foreach ($file in $manifest.files) {
    if ($file.path -cnotmatch '^[A-Za-z0-9_.-]+$') { throw 'Evidence path refused' }
    $path=Join-Path $PSScriptRoot $file.path
    if ((Get-Item -LiteralPath $path).Length -ne $file.bytes -or (Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant() -cne $file.sha256) { throw "Evidence drift: $($file.path)" }
}
$raw=Get-Content -Raw (Join-Path $PSScriptRoot 'host.log')
Check-Host $raw
foreach ($binary in $manifest.binaries) { if (-not $raw.Contains("$($binary.sha256)  tests-$($binary.compiler)")) { throw 'Binary digest differs' } }
foreach ($name in @('guard.sh','run.sh','screen.sh')) {
    $file=$manifest.files | Where-Object { $_.path -ceq $name }
    if (-not $raw.Contains("$($file.sha256)  $name")) { throw 'Executed script differs' }
}
$screen=Get-Content -Raw (Join-Path $PSScriptRoot 'screen.log')
if (-not $screen.Contains(@(Expected-Events)[0]) -or -not $screen.Contains('test result: ok. 1 passed; 0 failed;') -or
    -not $screen.Contains('HC074-ORIGINAL-DESCRIPTORS-SCREEN completed=true admission=false') -or
    $screen -cmatch 'FAILED|panicked|error:') { throw 'Healthy screen incomplete' }
$initial=Get-Content -Raw (Join-Path $PSScriptRoot 'initial-test-screen.log')
if (-not $initial.Contains('assertion `left == right` failed') -or -not $initial.Contains('test result: FAILED. 0 passed; 1 failed;')) { throw 'Initial oracle failure missing' }
$offset=Get-Content -Raw (Join-Path $PSScriptRoot 'offset-baseline-negative.log')
if (-not $offset.Contains('original_descriptor_capture_refuses_nonzero_sink_position') -or -not $offset.Contains('test result: FAILED. 0 passed; 1 failed;')) { throw 'Offset regression baseline missing' }
foreach ($gate in @(@('static.log','test result: ok. 207 passed; 0 failed;'),@('doc-check.log','doc-check: OK'),@('registry.log','184 discovered gates, all registered'),@('local-contract.log','OK (local, non-promotable)'))) {
    $log=Get-Content -Raw (Join-Path $PSScriptRoot $gate[0])
    if (-not $log.Contains($gate[1]) -or $log -cmatch 'FAILED|panicked|error:') { throw 'Focused gate incomplete' }
}
foreach ($compiler in @('194','188')) {
    $log=Get-Content -Raw (Join-Path $PSScriptRoot "$compiler-build.log")
    if (-not $log.Contains('test result: ok. 2 passed; 0 failed;') -or
        [regex]::Matches($log,'Finished `dev`').Count -ne 2 -or [regex]::Matches($log,'Finished `release`').Count -ne 1 -or
        $log -cmatch 'FAILED|panicked|error:|warning:') { throw 'Compiler gate incomplete' }
}
$repo=(& git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Repository unavailable' }
foreach ($source in $manifest.sources) {
    $blob=(& git -C $repo rev-parse "$($manifest.source_commit):$($source.path)").Trim()
    if ($LASTEXITCODE -ne 0 -or $blob -cne $source.blob) { throw 'Committed source drift' }
    $current=(& git -C $repo hash-object "--path=$($source.path)" (Join-Path $repo $source.path)).Trim()
    if ($LASTEXITCODE -ne 0 -or $current -cne $blob) { throw 'Working source drift' }
}
$buildInputs=@(Get-Content (Join-Path $PSScriptRoot 'source-hashes.log'))
if ($buildInputs.Count -ne 9) { throw 'Build source inventory differs' }
foreach ($line in $buildInputs) {
    if ($line -cnotmatch '^([a-f0-9]{64})  ([A-Za-z0-9_./-]+)$') { throw 'Invalid build source record' }
    $hash,$path=$Matches[1],$Matches[2]
    $source=@($manifest.sources | Where-Object { $_.path -ceq $path })
    if ($source.Count -ne 1 -or $source[0].raw_sha256 -cne $hash) { throw 'Saved build input differs' }
}
$info=[Diagnostics.ProcessStartInfo]::new('git')
$info.UseShellExecute=$false; $info.CreateNoWindow=$true
$info.RedirectStandardOutput=$true; $info.RedirectStandardError=$true
foreach ($arg in @('-C',$repo,'diff','--binary',"$($manifest.baseline_commit)..$($manifest.source_commit)")) { $info.ArgumentList.Add($arg) }
$process=[Diagnostics.Process]::Start($info)
$expected=$process.StandardOutput.ReadToEnd(); $errors=$process.StandardError.ReadToEnd(); $process.WaitForExit()
if ($process.ExitCode -ne 0 -or $errors -ne '' -or $expected -cne [IO.File]::ReadAllText((Join-Path $PSScriptRoot 'implementation.patch'))) { throw 'Committed capsule differs' }
& git -C $repo apply --reverse --check (Join-Path $PSScriptRoot 'implementation.patch')
if ($LASTEXITCODE -ne 0) { throw 'Capsule cannot reverse implementation' }
if ((Get-FileHash -LiteralPath (Join-Path $repo 'docs/testing/performance/0.74/qualification-manifest.toml')).Hash.ToLowerInvariant() -cne $manifest.qualification_sha256) { throw 'Qualification changed' }
Write-Output 'HC074-ORIGINAL-DESCRIPTORS-EVIDENCE verified=true controls=6 refusals=20 holder_cases=24 admission=false'
