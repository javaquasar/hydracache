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
function CheckStopped([string]$Raw) {
    Require ($Raw.Contains('Original(Challenge(Time))')) 'missing original challenge refusal'
    Require ($Raw.Contains('test result: FAILED. 0 passed; 1 failed;')) 'missing red screen'
    Require ($Raw.Contains('HC074-COLLECTOR-LOSS-COHORT compiler=194 completed=true')) 'current cohort incomplete'
    Require ($Raw.Contains('HC074-COLLECTOR-LOSS-COHORT compiler=188 begin=true')) 'missing MSRV screen'
    Require (-not $Raw.Contains('HC074-COLLECTOR-LOSS-COHORT compiler=188 completed=true')) 'false MSRV completion'
    Require (-not $Raw.Contains('HC074-COLLECTOR-LOSS-HOST completed=true')) 'false full completion'
    Require ([regex]::Matches($Raw,'HC074-COLLECTOR-LOSS-PHASE name=owner begin=true').Count -eq 1) 'faults ran after red screen'
    Require ([regex]::Matches($Raw,'authentication_calls: 34').Count -eq 1) 'refused frame not complete'
    Require ($Raw.Contains('signature_calls: 34') -and $Raw.Contains('first_time: None')) 'wrong refusal layer'
    foreach ($repeat in 1..3) {
        $row = "HC074-COLLECTOR-LOSS-NOTICE seed=0x0740c318 repeat=$repeat reason=Expired barred_consumed=true expiry_notice=E origins_exact=true sticky=true unchanged=true start=false"
        Require ([regex]::Matches($Raw,[regex]::Escape($row)).Count -eq 1) 'lost B count'
        foreach ($cut in @('before-frame','after-frame')) {
            foreach ($order in @('custodian-first','collector-first')) {
                $row = "HC074-COLLECTOR-LOSS-OWNER seed=0x0740c418 repeat=$repeat cut=$cut order=$order owner_signal=9 first_signal=9 last_signal=9 io_closed=4 survivor_fds=3 retained_past_original_bound=true unchanged=true replay_refused=true start=false"
                Require ([regex]::Matches($Raw,[regex]::Escape($row)).Count -eq 1) 'selected owner result'
            }
        }
    }
    Require ([regex]::Matches($Raw,'HC074-COLLECTOR-LOSS-CHECKPOINT ').Count -eq 12) 'checkpoint count'
    Require ([regex]::Matches($Raw,'test result: ok. 8 passed;').Count -eq 2) 'both parser/unit cohorts'
    Require ([regex]::Matches($Raw,'test result: ok. 32 passed;').Count -eq 2) 'both storage/unit cohorts'
    Require ($Raw.Contains('Ran 15 tests')) 'Python units'
}
function CheckAttribution([string]$Raw) {
    $rows = @('1 role=previous exit=101','2 role=current exit=0','3 role=current exit=0','4 role=previous exit=0')
    foreach ($row in $rows) {
        Require ($Raw.Contains("HC074-COLLECTOR-HEALTHY-ATTRIBUTION seed=0x0740c518 sample=$row admission=false")) 'ABBA status'
    }
    Require ($Raw.Contains('Original(Challenge(Time))')) 'baseline refusal differs'
    Require ($Raw.Contains('stopped_matrix_not_replaced=true admission=false')) 'attribution promoted'
}
function CheckManifest($Manifest) {
    Require ($Manifest.schema -ceq 'hc074-original-collector-loss-stopped-v1') 'schema'
    Require ($Manifest.admission -ceq $false -and $Manifest.msrv_faults_completed -ceq $false) 'false admission'
    Require ($Manifest.candidate_committed -ceq $false) 'false committed candidate'
    Require ($Manifest.status -ceq 'withdrawn-msrv-healthy-refusal') 'status'
    Require ($Manifest.completed_owner_cases -eq 12 -and $Manifest.lost_b_controls -eq 3) 'false result counts'
    Require ($Manifest.source_files.Count -eq 13 -and $Manifest.build_inputs.Count -eq 323) 'source inventory'
    Require ($Manifest.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification'
    Require ($Manifest.scratch -ceq '/tmp/hc074-collector-loss-matrix-20261018.mPInXP') 'scratch'
}
$manifest = Get-Content (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
CheckManifest $manifest
$actual = @(Get-ChildItem -LiteralPath $packet -File -Recurse | ForEach-Object {
    [IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/')
} | Where-Object {$_ -ne 'manifest.json'} | Sort-Object)
Require (($actual -join "`n") -ceq (($manifest.files.path | Sort-Object) -join "`n")) 'closed file inventory'
foreach ($file in $manifest.files) {
    Require ($file.path -cnotmatch '(^/|\\|\.\.)') 'unsafe packet path'
    Require ((Get-FileHash -LiteralPath (Join-Path $packet $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) "file digest $($file.path)"
}
$sources = @{}
foreach ($file in $manifest.source_files) {
    $name = Join-Path $packet ('source-capsule/'+$file.path.Replace('/','__')+'.source')
    Require ((Get-FileHash -LiteralPath $name -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.raw_sha256) 'raw capsule'
    $lf = [IO.File]::ReadAllText($name).Replace("`r`n","`n")
    Require ((HashText $lf) -ceq $file.lf_sha256) 'canonical capsule'
    Require (-not $sources.ContainsKey($file.path)) 'duplicate source'
    $sources[$file.path] = $lf
}
foreach ($file in $manifest.build_inputs) {
    $lf = if ($sources.ContainsKey($file.path)) {$sources[$file.path]} else {GitText @('show',"$($manifest.base_commit):$($file.path)")}
    Require ((HashText $lf) -ceq $file.lf_sha256) "base/capsule input $($file.path)"
}
$inputRows = @($manifest.build_inputs | ForEach-Object {"$($_.raw_sha256)  $($_.path)"})
Require ([IO.File]::ReadAllText((Join-Path $packet 'build-inputs-final.log')) -ceq (($inputRows -join "`n")+"`n")) 'raw input binding'
$hostLog = [IO.File]::ReadAllText((Join-Path $packet 'host-first-red.log'))
$attribution = [IO.File]::ReadAllText((Join-Path $packet 'healthy-attribution.log'))
CheckStopped $hostLog
CheckAttribution $attribution
foreach ($compiler in @('194','188')) {
    $hash = (Get-Content (Join-Path $packet "binary-$compiler.sha256") -Raw).Trim()
    Require ($hostLog.Contains("$hash  ./tests-$compiler")) 'candidate binary identity'
}
foreach ($script in @('guard.sh','run.sh','original_collector_owner_loss.py','test_original_collector_owner_loss.py','original_custody_owner_loss.py','test_original_custody_owner_loss.py')) {
    $hash = (Get-FileHash -LiteralPath (Join-Path $packet $script) -Algorithm SHA256).Hash.ToLowerInvariant()
    Require ($hostLog.Contains("$hash  ./$script")) 'host script identity'
}
$hash = (Get-FileHash -LiteralPath (Join-Path $packet 'healthy-attribution.sh') -Algorithm SHA256).Hash.ToLowerInvariant()
Require ($attribution.Contains("$hash  ./healthy-attribution.sh")) 'ABBA script'
Require ($attribution.Contains('33ede34173e7b73c53f09f3ef55f3ff971e55a7398b2dcf224f67c03c71d4796')) 'baseline binary'
$postflight = [IO.File]::ReadAllText((Join-Path $packet 'postflight.log'))
Require ($postflight.Contains('postflight=ok no_fixture_executable=true supervisor_unchanged=true admission=false')) 'postflight'
Require ($postflight.Contains('preflight=ok supervisor_pid=8839 restarts=0')) 'stable service'
Require ($postflight -cnotmatch 'unexpected_executable=|FAILED') 'leftover fixture'
Require ((Get-FileHash -LiteralPath (Join-Path $root 'docs/testing/performance/0.74/qualification-manifest.toml') -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $manifest.qualification_sha256) 'qualification drift'
if ($SelfTest) {
    $negatives = 0
    foreach ($bad in @($hostLog.Replace('Original(Challenge(Time))','other'),
        $hostLog.Replace('expiry_notice=E','expiry_notice=B'),
        $hostLog.Replace('last_signal=9','last_signal=0'),
        $hostLog.Replace('authentication_calls: 34','authentication_calls: 33'),
        ($hostLog+"`nHC074-COLLECTOR-LOSS-COHORT compiler=188 completed=true`n"))) {
        try {CheckStopped $bad} catch {$negatives++; continue}; throw 'stopped mutation escaped'
    }
    try {CheckAttribution ($attribution.Replace('role=previous exit=101','role=previous exit=0'))} catch {$negatives++}
    Require ($negatives -eq 6) 'ABBA refusal mutation escaped'
    foreach ($kind in @('admission','msrv','counts')) {
        $bad = $manifest | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        switch ($kind) { admission {$bad.admission=$true} msrv {$bad.msrv_faults_completed=$true} counts {$bad.completed_owner_cases=24} }
        try {CheckManifest $bad} catch {$negatives++; continue}; throw 'manifest mutation escaped'
    }
    "HC074-COLLECTOR-LOSS-STOPPED-OFFLINE positive=1 negatives=$negatives full_gate=false admission=false"
} else {'HC074-COLLECTOR-LOSS-STOPPED-OFFLINE verified=true full_gate=false admission=false'}
