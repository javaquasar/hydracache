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
    Require ([regex]::Matches($Raw,'HC074-BARRED-COLLECTOR-CONTROL sample=[0-2] create_ipc=34 validate_ipc=34 collector_fds=7 custody_fds=3 mask_exact=true unchanged=true start=false').Count -eq 8) 'healthy controls'
    Require ([regex]::Matches($Raw,'HC074-BARRED-COLLECTOR-REFUSAL seed=0x0740c118 case=').Count -eq 18) 'refusals'
    foreach ($case in @('clone-error','restore-error','trust','output','message','custody-flags','collector-kill','custodian-kill','final-expiry')) {
        Require ([regex]::Matches($Raw,"case=$case reason=.+ mask_exact=true sticky=true unchanged=true start=false").Count -eq 2) "refusal $case"
    }
    foreach ($case in @('expiry','peer-activity','peer-loss')) {
        foreach ($repeat in 1..3) {
            $row = "HC074-BARRED-COLLECTOR-PROCESS seed=0x0740c218 repeat=$repeat case=$case collector_fds=3 io_closed=4 custody_killed_first=true fence_retained=true unchanged=true start=false"
            Require ([regex]::Matches($Raw,[regex]::Escape($row)).Count -eq 2) "process $case/$repeat"
        }
    }
    foreach ($compiler in @('194','188')) {
        $begin = "HC074-BARRED-COLLECTOR-COHORT compiler=$compiler begin=true"
        $end = "HC074-BARRED-COLLECTOR-COHORT compiler=$compiler completed=true"
        Require ([regex]::Matches($Raw,[regex]::Escape($begin)).Count -eq 1) 'cohort begin'
        Require ([regex]::Matches($Raw,[regex]::Escape($end)).Count -eq 1) 'cohort end'
        $cohort = $Raw.Substring($Raw.IndexOf($begin),$Raw.IndexOf($end)-$Raw.IndexOf($begin))
        foreach ($phase in @('screen','healthy','refusals','process_matrix')) {
            Require ($cohort.Contains("HC074-BARRED-COLLECTOR-PHASE name=$phase completed=true")) 'missing phase'
        }
        Require ($cohort.Contains('test result: ok. 5 passed;')) 'descriptor units'
        Require ($cohort.Contains('test result: ok. 32 passed;')) 'storage units'
    }
    Require ($Raw.Contains('HC074-BARRED-COLLECTOR-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false')) 'postflight'
    Require ([regex]::Matches($Raw,'preflight=ok supervisor_pid=8839 restarts=0').Count -eq 5) 'host guards'
}
function CheckManifest($Manifest) {
    Require ($Manifest.schema -eq 'hc074-original-barred-collector-v1') 'schema'
    Require ($Manifest.admission -ceq $false) 'admission'
    Require ($Manifest.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification'
    Require ($Manifest.source_commit -cmatch '^[0-9a-f]{40}$') 'source'
    Require ($Manifest.source_files.Count -eq 10) 'source inventory'
    Require ($Manifest.build_inputs.Count -eq 320) 'input inventory'
    Require (@($Manifest.build_inputs.path | Sort-Object -Unique).Count -eq 320) 'duplicate input'
    Require ($Manifest.scratch -ceq '/tmp/hc074-barred-collector-20261018.ET1hWc') 'scratch'
}
$manifest = Get-Content (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
CheckManifest $manifest
$actual = @(Get-ChildItem -LiteralPath $packet -File -Recurse | ForEach-Object {
    [IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/')
} | Where-Object {$_ -ne 'manifest.json'} | Sort-Object)
Require (($actual -join "`n") -ceq (($manifest.files.path | Sort-Object) -join "`n")) 'file inventory'
foreach ($file in $manifest.files) {
    Require ($file.path -cnotmatch '(^/|\\|\.\.)') 'unsafe packet path'
    Require ((Get-FileHash -LiteralPath (Join-Path $packet $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) "file digest $($file.path)"
}
$changes = @((GitText @('diff','--name-only',$manifest.base_commit,$manifest.source_commit)).Trim() -split "`n")
Require ((($changes | Sort-Object) -join "`n") -ceq (($manifest.source_files.path | Sort-Object) -join "`n")) 'source changes'
foreach ($file in $manifest.source_files) {
    Require ((HashText (GitText @('show',"$($manifest.source_commit):$($file.path)"))) -ceq $file.sha256) 'source binding'
}
foreach ($file in $manifest.build_inputs) {
    Require ($file.raw_sha256 -cmatch '^[0-9a-f]{64}$') 'raw input digest'
    Require ((HashText (GitText @('show',"$($manifest.source_commit):$($file.path)"))) -ceq $file.lf_sha256) 'input source binding'
}
$inputLines = @($manifest.build_inputs | ForEach-Object { "$($_.raw_sha256)  $($_.path)" })
Require ([IO.File]::ReadAllText((Join-Path $packet 'build-inputs-final.log')) -ceq (($inputLines -join "`n")+"`n")) 'raw input log binding'
Require ([IO.File]::ReadAllText((Join-Path $packet 'implementation.patch')) -ceq (GitText @('diff','--binary',$manifest.base_commit,$manifest.source_commit))) 'patch binding'
Require ((Get-FileHash -LiteralPath (Join-Path $root 'docs/testing/performance/0.74/qualification-manifest.toml') -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $manifest.qualification_sha256) 'qualification drift'
$hostLog = [IO.File]::ReadAllText((Join-Path $packet 'host.log'))
CheckHost $hostLog
foreach ($compiler in @('194','188')) {
    $hash = (Get-Content (Join-Path $packet "binary-$compiler.sha256") -Raw).Trim()
    Require ($hash -cmatch '^[0-9a-f]{64}$') 'binary identity'
    Require ($hostLog.Contains("$hash  ./tests-$compiler")) 'host binary mismatch'
}
foreach ($script in @('guard.sh','run.sh')) {
    $hash = (Get-FileHash -LiteralPath (Join-Path $packet $script) -Algorithm SHA256).Hash.ToLowerInvariant()
    Require ($hostLog.Contains("$hash  ./$script")) 'host script mismatch'
}
if ($SelfTest) {
    $negatives = 0
    foreach ($bad in @(
        $hostLog.Replace('create_ipc=34','create_ipc=33'),
        $hostLog.Replace('custody_killed_first=true','custody_killed_first=false'),
        $hostLog.Replace('supervisor_pid=8839','supervisor_pid=0'),
        $hostLog.Replace('name=screen completed=true','name=screen completed=false'),
        ($hostLog+"`nFAILED`n")
    )) {
        try {CheckHost $bad} catch {$negatives++; continue}; throw 'host mutation escaped'
    }
    foreach ($kind in @('admission','inputs','qualification')) {
        $bad = $manifest | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        switch ($kind) {
            admission {$bad.admission=$true}
            inputs {$bad.build_inputs=@($bad.build_inputs | Select-Object -Skip 1)}
            qualification {$bad.qualification_sha256='0'*64}
        }
        try {CheckManifest $bad} catch {$negatives++; continue}; throw 'manifest mutation escaped'
    }
    "HC074-BARRED-COLLECTOR-OFFLINE positive=1 negatives=$negatives admission=false"
} else { 'HC074-BARRED-COLLECTOR-OFFLINE verified=true admission=false' }
