param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$root = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
function Require([bool]$Condition,[string]$Reason) { if (-not $Condition) { throw $Reason } }
function HashText([string]$Value) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Value))).ToLowerInvariant()
}
function GitText([string[]]$Arguments) {
    $start = [Diagnostics.ProcessStartInfo]::new('git')
    $start.WorkingDirectory = $root
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
    foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($start)
    $value = $process.StandardOutput.ReadToEnd()
    $process.WaitForExit()
    Require ($process.ExitCode -eq 0) 'git failed'
    $value
}
function CheckHost([string]$Raw) {
    Require ($Raw -cnotmatch 'FAILED|panicked|Traceback|unexpected_executable=') 'host failure'
    $controls = [regex]::Matches($Raw,'HC074-BARRED-CUSTODY-CONTROL sample=0 create_ipc=34 validate_ipc=34 inherited=3 mask_exact=true unchanged=true start=false')
    Require ($controls.Count -eq 2) 'missing compiler controls'
    $notices = [regex]::Matches($Raw,'HC074-CUSTODY-LOSS-NOTICE seed=0x0740be18 repeat=([123]) reason=Expired pending_consumed=true expiry_notice=E origins_exact=true sticky=true unchanged=true start=false')
    Require ($notices.Count -eq 6) 'missing lost notices'
    foreach ($repeat in 1..3) { Require (@($notices | Where-Object { $_.Groups[1].Value -eq "$repeat" }).Count -eq 2) 'notice repetition' }
    $owners = [regex]::Matches($Raw,'HC074-CUSTODY-LOSS-OWNER seed=0x0740bf18 repeat=([123]) cut=(before-frame|after-frame) owner_signal=9 custodian_signal=9 inherited=3 retained_past_original_bound=true unchanged=true replay_refused=true start=false')
    Require ($owners.Count -eq 12) 'missing owner deaths'
    $checkpoints = [regex]::Matches($Raw,'(?m)^HC074-CUSTODY-LOSS-CHECKPOINT (\{.+\})\r?$')
    Require ($checkpoints.Count -eq 12) 'missing custody checkpoints'
    foreach ($checkpoint in $checkpoints) {
        $point = $checkpoint.Groups[1].Value | ConvertFrom-Json
        Require ($point.schema -eq 'hc074-pending-owner-test-v1' -and $point.owner -gt 0 -and $point.custodian -gt 0) 'checkpoint identity'
        Require ($point.cut -in 'before-frame','after-frame' -and $point.origins.Count -eq 6 -and $point.deadline -gt $point.origins[5]) 'checkpoint origins'
        Require ($point.closing_authentication -eq $(if ($point.cut -eq 'before-frame') { 0 } else { 34 })) 'checkpoint frame'
    }
    foreach ($repeat in 1..3) {
        foreach ($cut in 'before-frame','after-frame') {
            Require (@($owners | Where-Object { $_.Groups[1].Value -eq "$repeat" -and $_.Groups[2].Value -eq $cut }).Count -eq 2) 'owner repetition'
        }
    }
    foreach ($compiler in '194','188') {
        $begin = "HC074-CUSTODY-LOSS-COHORT compiler=$compiler begin=true"
        $end = "HC074-CUSTODY-LOSS-COHORT compiler=$compiler completed=true"
        Require ($Raw.IndexOf($begin) -ge 0 -and $Raw.IndexOf($end) -gt $Raw.IndexOf($begin)) 'compiler completion'
        $cohort = $Raw.Substring($Raw.IndexOf($begin), $Raw.IndexOf($end)-$Raw.IndexOf($begin))
        $binary = [regex]::Match($Raw,"(?m)^([0-9a-f]{64})  ./tests-$compiler\r?$").Groups[1].Value
        Require ($binary.Length -eq 64 -and $cohort.Contains("HC074-CUSTODY-LOSS-DRIVER binary_sha256=$binary")) 'binary cohort'
        $previous = -1
        foreach ($phase in 'screen','notice','owner') {
            foreach ($state in 'begin','completed') {
                $position = $cohort.IndexOf("HC074-CUSTODY-LOSS-PHASE name=$phase $state=true")
                Require ($position -gt $previous) 'phase order'
                $previous = $position
            }
        }
    }
    Require ($Raw.Contains('HC074-CUSTODY-LOSS-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false')) 'host closure'
}
$manifest = Get-Content -Raw (Join-Path $packet 'manifest.json') | ConvertFrom-Json
Require ($manifest.schema -eq 'hc074-original-custody-loss-v1' -and -not $manifest.admission) 'schema/admission'
$actual = @(Get-ChildItem -LiteralPath $packet -File -Recurse | ForEach-Object {
    [IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/')
} | Where-Object { $_ -ne 'manifest.json' } | Sort-Object)
Require (($actual -join "`n") -ceq (@($manifest.files.path | Sort-Object) -join "`n")) 'packet inventory'
foreach ($file in $manifest.files) {
    Require ($file.path -notmatch '(^/|\.\.|\\)' -and $file.sha256 -match '^[0-9a-f]{64}$') 'unsafe artifact'
    Require ((Get-FileHash -LiteralPath (Join-Path $packet $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -eq $file.sha256) "artifact changed: $($file.path)"
}
Require (@($manifest.source_files).Count -eq 10) 'source count'
$sourcePaths = @((GitText @('diff','--name-only',$manifest.base_commit,$manifest.source_commit)).Trim() -split "`n" | Sort-Object)
Require (($sourcePaths -join "`n") -ceq (@($manifest.source_files.path | Sort-Object) -join "`n")) 'source inventory'
foreach ($file in $manifest.source_files) {
    Require ((HashText (GitText @('show',"$($manifest.source_commit):$($file.path)"))) -eq $file.sha256) 'source identity'
}
Require ((GitText @('diff','--binary',$manifest.base_commit,$manifest.source_commit)) -ceq
    [IO.File]::ReadAllText((Join-Path $packet 'implementation.patch'))) 'source patch'
Require (@($manifest.build_inputs).Count -eq 316) 'input count'
$expectedInputs = @((GitText @('ls-tree','-r','--name-only',$manifest.source_commit,'tools/long-run-supervisor-074/src')).Trim() -split "`n" | Where-Object { $_ -match '^tools/long-run-supervisor-074/src/[^/]+\.rs$' })
$expectedInputs += 'Cargo.lock','Cargo.toml','tools/long-run-supervisor-074/Cargo.toml'
Require ((@($expectedInputs | Sort-Object) -join "`n") -ceq (@($manifest.build_inputs.path | Sort-Object) -join "`n")) 'input inventory'
foreach ($entry in $manifest.build_inputs) {
    Require ((HashText (GitText @('show',"$($manifest.source_commit):$($entry.path)"))) -eq $entry.lf_sha256) 'input provenance'
    Require ([IO.File]::ReadAllText((Join-Path $packet 'build-inputs-final.log')).Contains("$($entry.raw_sha256)  $($entry.path)")) 'raw input inventory'
}
Require ((Get-FileHash -LiteralPath (Join-Path $root 'docs/testing/performance/0.74/qualification-manifest.toml') -Algorithm SHA256).Hash.ToLowerInvariant() -eq $manifest.qualification_sha256) 'qualification changed'
$hostLog = [IO.File]::ReadAllText((Join-Path $packet 'host.log'))
CheckHost $hostLog
Require ([IO.File]::ReadAllText((Join-Path $packet 'postflight.log')).Contains('HC074-CUSTODY-LOSS-POSTFLIGHT unchanged=true negative_and_positive_absent=true admission=false')) 'positive postflight'
foreach ($name in 'guard.sh','run.sh','original_custody_owner_loss.py','test_original_custody_owner_loss.py') {
    $digest = (Get-FileHash -LiteralPath (Join-Path $packet $name) -Algorithm SHA256).Hash.ToLowerInvariant()
    Require ($hostLog.Contains("$digest  ./$name")) 'host script identity'
}
Require ([IO.File]::ReadAllText((Join-Path $packet 'static-final.log')).Contains('216 passed; 0 failed')) 'static contracts'
$negativeRoot = Join-Path $packet 'negative-first-oracle'
$negative = Get-Content -Raw (Join-Path $negativeRoot 'source-manifest.json') | ConvertFrom-Json
Require (@($negative.files).Count -eq 10) 'negative source count'
foreach ($file in $negative.files) {
    Require ((Get-FileHash -LiteralPath (Join-Path $negativeRoot $file.copy) -Algorithm SHA256).Hash.ToLowerInvariant() -eq $file.sha256) 'negative source identity'
}
$negativeLog = [IO.File]::ReadAllText((Join-Path $negativeRoot 'host.log'))
Require ($negativeLog.Contains('HC074-CUSTODY-LOSS-NOTICE seed=0x0740be18 repeat=1 reason=Expired') -and
    $negativeLog.Contains('test result: FAILED. 0 passed; 1 failed') -and
    -not $negativeLog.Contains('HC074-CUSTODY-LOSS-PHASE name=owner begin=true')) 'negative scope'
Require ([IO.File]::ReadAllText((Join-Path $negativeRoot 'postflight.log')).Contains('no_fixture_executable=true')) 'negative cleanup'
$originRoot = Join-Path $packet 'negative-authority-origin'
$originFailure = Get-Content -Raw (Join-Path $originRoot 'source-manifest.json') | ConvertFrom-Json
Require (@($originFailure.files).Count -eq 10) 'authority negative source count'
foreach ($file in $originFailure.files) {
    Require ((Get-FileHash -LiteralPath (Join-Path $originRoot $file.copy) -Algorithm SHA256).Hash.ToLowerInvariant() -eq $file.sha256) 'authority negative source identity'
}
$originLog = [IO.File]::ReadAllText((Join-Path $originRoot 'host.log'))
Require ($originLog.Contains('diagnostic_worker_trust_input_tests.rs:587:86') -and $originLog.Contains('TimeoutError: timed out') -and
    -not $originLog.Contains('HC074-CUSTODY-LOSS-OWNER seed=')) 'authority negative scope'
Require ([IO.File]::ReadAllText((Join-Path $originRoot 'postflight.log')).Contains('no_fixture_executable=true')) 'authority negative cleanup'
$tracer = Join-Path $packet 'negative-tracer'
$traceLog = [IO.File]::ReadAllText((Join-Path $tracer 'trace.log'))
Require ($traceLog.Contains('diagnostic_collector_starting_publication_tests.rs:116:10') -and $traceLog.Contains('value: Time') -and
    -not $traceLog.Contains('HC074-CUSTODY-LOSS-TRACE compiler=194 completed=true') -and
    -not $traceLog.Contains('HC074-CUSTODY-LOSS-TRACE compiler=188 begin=true')) 'tracer negative scope'
Require (-not [IO.File]::ReadAllText((Join-Path $tracer 'pending-recv-194.log')).Contains('"P"')) 'tracer reached pending'
Require ([IO.File]::ReadAllText((Join-Path $tracer 'postflight.log')).Contains('negative_and_positive_absent=true')) 'tracer cleanup'
if ($SelfTest) {
    $mutations = @(
        $hostLog.Replace('reason=Expired','reason=accepted'),
        $hostLog.Replace('cut=before-frame','cut=unknown'),
        $hostLog.Replace('seed=0x0740bf18','seed=0x00000000'),
        $hostLog.Replace('owner_signal=9','owner_signal=0'),
        $hostLog.Replace('compiler=188 completed=true','compiler=188 incomplete=true'),
        $hostLog.Replace('name=screen completed=true','name=screen incomplete=true'),
        $hostLog.Replace('no_fixture_executable=true','no_fixture_executable=false'),
        $hostLog.Replace('HC074-CUSTODY-LOSS-DRIVER binary_sha256=','HC074-CUSTODY-LOSS-DRIVER unknown_sha256='),
        ($hostLog + "`nTraceback`n")
    )
    foreach ($mutation in $mutations) {
        $refused = $false
        try { CheckHost $mutation } catch { $refused = $true }
        Require $refused 'negative verifier control accepted'
    }
    'HC074-CUSTODY-LOSS verification=true selftest_positive=1 selftest_negative=9 admission=false'
} else { 'HC074-CUSTODY-LOSS verification=true admission=false' }
