param([switch]$SelfTest)
$ErrorActionPreference = 'Stop'
$packet = $PSScriptRoot
$root = (Resolve-Path (Join-Path $packet '../../../../../..')).Path
function Require([bool]$Condition,[string]$Reason) {if (-not $Condition) {throw $Reason}}
function HashText([string]$Value) {
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($Value))).ToLowerInvariant()
}
function GitText([string]$Ref,[string]$Path) {
    $start = [Diagnostics.ProcessStartInfo]::new('git')
    $start.WorkingDirectory=$root; $start.UseShellExecute=$false
    $start.RedirectStandardOutput=$true; $start.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
    $start.ArgumentList.Add('show'); $start.ArgumentList.Add("${Ref}:$Path")
    $process=[Diagnostics.Process]::Start($start)
    $value=$process.StandardOutput.ReadToEnd(); $process.WaitForExit()
    Require ($process.ExitCode -eq 0) 'git source unavailable'
    $value
}
function CheckManifest($m) {
    Require ($m.schema -ceq 'hc074-original-enrollment-frame-v1') 'schema'
    Require ($m.status -ceq 'verified-barred-only' -and $m.full_selected_gate -ceq $true) 'selected gate'
    Require ($m.admission -ceq $false -and $m.original_start -ceq $false -and $m.product_workload -ceq $false) 'false admission'
    Require ($m.healthy_controls -eq 8 -and $m.live_refusals -eq 18 -and $m.lost_b_controls -eq 6 -and $m.owner_cases -eq 24 -and $m.selected_sigkill_signals -eq 72) 'result inventory'
    Require ($m.source_files.Count -eq 14 -and $m.build_inputs.Count -eq 326) 'source inventory'
    Require ($m.source_commit -cmatch '^[a-f0-9]{40}$' -and $m.base_commit -cmatch '^27dbbac7[a-f0-9]{32}$') 'source identity'
    Require ($m.scratch -ceq '/tmp/hc074-enrollment-frame-20261018.jj6xm5') 'scratch'
    Require ($m.qualification_sha256 -ceq '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc') 'qualification identity'
}
function Count([string]$Raw,[string]$Needle) {[regex]::Matches($Raw,[regex]::Escape($Needle)).Count}
function CheckHost([string]$Raw) {
    Require ($Raw -cnotmatch 'FAILED|panicked|Traceback|unexpected_executable=') 'unexpected failure'
    Require ((Count $Raw 'HC074-ENROLLMENT-FRAME-HOST completed=true supervisor_unchanged=true no_fixture_executable=true admission=false') -eq 1) 'host completion'
    Require ((Count $Raw 'test result: ok. 8 passed;') -eq 2 -and (Count $Raw 'test result: ok. 32 passed;') -eq 2) 'fast/storage cohorts'
    Require ($Raw.Contains('Ran 15 tests')) 'Python corpus'
    Require ((Count $Raw 'HC074-ENROLLMENT-CONTROL sample=') -eq 8) 'healthy count'
    Require ((Count $Raw 'create_ipc=34 validate_ipc=34 collector_fds=7 custody_fds=3 unchanged=true start=false') -eq 8) 'complete frame counters'
    foreach ($compiler in @('194','188')) {
        Require ((Count $Raw "HC074-ENROLLMENT-FRAME-SCREEN compiler=$compiler completed=true") -eq 1) 'screen'
        $begin="HC074-ENROLLMENT-FRAME-COHORT compiler=$compiler begin=true"
        $end="HC074-ENROLLMENT-FRAME-COHORT compiler=$compiler completed=true"
        Require ((Count $Raw $begin) -eq 1 -and (Count $Raw $end) -eq 1) 'cohort'
        $part=$Raw.Substring($Raw.IndexOf($begin),$Raw.IndexOf($end)-$Raw.IndexOf($begin))
        foreach ($case in @('clone-error','restore-error','trust','output','message','custody-flags','collector-kill','custodian-kill','final-expiry')) {
            Require ((Count $part "HC074-ENROLLMENT-REFUSAL seed=0x0740e118 case=$case reason=") -eq 1) 'refusal inventory'
        }
        foreach ($repeat in 1..3) {
            $row="HC074-ENROLLMENT-LOSS-NOTICE seed=0x0740e318 repeat=$repeat reason=Expired barred_consumed=true expiry_notice=E origins_exact=true sticky=true unchanged=true start=false"
            Require ((Count $part $row) -eq 1) 'real B loss'
            foreach ($cut in @('before-frame','after-frame')) {foreach ($order in @('custodian-first','collector-first')) {
                $row="HC074-ENROLLMENT-LOSS-OWNER seed=0x0740e418 repeat=$repeat cut=$cut order=$order owner_signal=9 first_signal=9 last_signal=9 io_closed=4 survivor_fds=3 retained_past_original_bound=true unchanged=true replay_refused=true start=false"
                Require ((Count $part $row) -eq 1) 'owner fault result'
            }}
        }
        Require ((Count $part 'HC074-ENROLLMENT-LOSS-CHECKPOINT ') -eq 12) 'checkpoint inventory'
        foreach ($line in ($part -split "`n" | Where-Object {$_.StartsWith('HC074-ENROLLMENT-LOSS-CHECKPOINT ')})) {
            $point=$line.Substring('HC074-ENROLLMENT-LOSS-CHECKPOINT '.Length) | ConvertFrom-Json
            Require ($point.schema -ceq 'hc074-enrollment-owner-test-v1' -and ($point.roles -join ',') -ceq 'collector,custodian') 'checkpoint protocol'
            Require ($point.frame_closed -is [bool] -and $point.frame_closed -ceq ($point.cut -ceq 'after-frame')) 'closing cut'
            Require ($point.origins.Count -eq 6 -and $point.collector -ne $point.custodian) 'original topology'
            $bound=[Math]::Min([Math]::Min([long]$point.origins[0]+300000000000,[long]$point.origins[3]+60000000000),[long]$point.origins[2]+10000000000)
            Require ($point.deadline -eq $bound) 'original absolute bound'
        }
    }
}
$m=Get-Content (Join-Path $packet 'manifest.json') -Raw | ConvertFrom-Json
CheckManifest $m
$actual=@(Get-ChildItem -LiteralPath $packet -Recurse -File | ForEach-Object {
    [IO.Path]::GetRelativePath($packet,$_.FullName).Replace('\','/')
} | Where-Object {$_ -ne 'manifest.json'} | Sort-Object)
Require (($actual -join "`n") -ceq (($m.files.path | Sort-Object) -join "`n")) 'closed file inventory'
foreach ($file in $m.files) {
    Require ($file.path -cnotmatch '(^/|\\|\.\.)') 'unsafe packet path'
    Require ((Get-FileHash -LiteralPath (Join-Path $packet $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.sha256) "file digest $($file.path)"
}
$sources=@{}
foreach ($file in $m.source_files) {
    Require (-not $sources.ContainsKey($file.path)) 'duplicate capsule'
    $name=Join-Path $packet ('source-capsule/'+$file.path.Replace('/','__')+'.source')
    Require ((Get-FileHash -LiteralPath $name -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $file.raw_sha256) 'raw capsule'
    $lf=[IO.File]::ReadAllText($name).Replace("`r`n","`n")
    Require ((HashText $lf) -ceq $file.lf_sha256) 'canonical capsule'
    Require ((GitText $m.source_commit $file.path).Replace("`r`n","`n") -ceq $lf) 'accepted source differs'
    $sources[$file.path]=$lf
}
$changed=@(git -C $root diff --name-only $m.base_commit $m.source_commit)
Require ($LASTEXITCODE -eq 0) 'source tree comparison failed'
Require ((($changed | Sort-Object) -join "`n") -ceq (($m.source_files.path | Sort-Object) -join "`n")) 'source changes outside capsule'
$parent=(git -C $root rev-parse "$($m.source_commit)^").Trim()
Require ($LASTEXITCODE -eq 0 -and $parent -ceq $m.base_commit) 'source parent differs'
foreach ($file in $m.build_inputs) {
    $lf=if ($sources.ContainsKey($file.path)) {$sources[$file.path]} else {GitText $m.base_commit $file.path}
    Require ((HashText $lf) -ceq $file.lf_sha256) "build input $($file.path)"
}
$rows=@($m.build_inputs | ForEach-Object {"$($_.raw_sha256)  $($_.path)"})
Require ([IO.File]::ReadAllText((Join-Path $packet 'build-inputs.log')) -ceq (($rows -join "`n")+"`n")) 'raw build input binding'
$hostLog=[IO.File]::ReadAllText((Join-Path $packet 'host.log'))
CheckHost $hostLog
foreach ($compiler in @('194','188')) {
    $hash=(Get-Content (Join-Path $packet "binary-$compiler.sha256") -Raw).Trim()
    Require ($hostLog.Contains("$hash  ./tests-$compiler")) 'image identity'
    Require ($hostLog.Contains("HC074-ENROLLMENT-LOSS-DRIVER binary_sha256=$hash")) 'owner driver image identity'
}
foreach ($script in @('guard.sh','run.sh','original_enrollment_owner_loss.py','test_original_enrollment_owner_loss.py','original_custody_owner_loss.py','test_original_custody_owner_loss.py')) {
    $hash=(Get-FileHash -LiteralPath (Join-Path $packet $script) -Algorithm SHA256).Hash.ToLowerInvariant()
    Require ($hostLog.Contains("$hash  ./$script")) "host script identity $script"
}
foreach ($script in @('original_enrollment_owner_loss.py','test_original_enrollment_owner_loss.py')) {
    $executed=[IO.File]::ReadAllText((Join-Path $packet $script)).Replace("`r`n","`n")
    $accepted=$sources["tools/long-run-supervisor-074/tests/$script"]
    Require ($executed.TrimEnd("`n") -ceq $accepted.TrimEnd("`n")) 'script change beyond EOF newline normalization'
}
$post=[IO.File]::ReadAllText((Join-Path $packet 'postflight.log'))
Require ($post.Contains('postflight=ok no_fixture_executable=true supervisor_unchanged=true admission=false') -and $post.Contains('preflight=ok supervisor_pid=8839 restarts=0')) 'stable postflight'
Require ((Get-FileHash -LiteralPath (Join-Path $root 'docs/testing/performance/0.74/qualification-manifest.toml') -Algorithm SHA256).Hash.ToLowerInvariant() -ceq $m.qualification_sha256) 'qualification drift'
Require ([IO.File]::ReadAllText((Join-Path $packet 'contracts.log')).Contains('225 passed; 0 failed;')) 'static contracts'
if ($SelfTest) {
    $negative=0
    foreach ($bad in @($hostLog.Replace('expiry_notice=E','expiry_notice=B'),
        $hostLog.Replace('last_signal=9','last_signal=0'),
        $hostLog.Replace('create_ipc=34','create_ipc=33'),
        $hostLog.Replace('HC074-ENROLLMENT-FRAME-SCREEN compiler=188 completed=true','missing'),
        ($hostLog+"`nFAILED`n"))) {
        try {CheckHost $bad} catch {$negative++; continue}; throw 'host mutation escaped'
    }
    foreach ($kind in @('admission','start','counts')) {
        $bad=$m | ConvertTo-Json -Depth 12 | ConvertFrom-Json
        switch ($kind) {admission {$bad.admission=$true} start {$bad.original_start=$true} counts {$bad.owner_cases=12}}
        try {CheckManifest $bad} catch {$negative++; continue}; throw 'manifest mutation escaped'
    }
    "HC074-ENROLLMENT-FRAME-OFFLINE positive=1 negatives=$negative selected_gate=true admission=false"
} else {'HC074-ENROLLMENT-FRAME-OFFLINE verified=true selected_gate=true admission=false'}
