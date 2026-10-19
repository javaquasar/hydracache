param([Parameter(Mandatory)][string]$Source,[Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut=Join-Path $PSScriptRoot $Label
if(Test-Path -LiteralPath $taskOut){throw 'Retain existing cohort'}
New-Item -ItemType Directory -Path $taskOut | Out-Null
Push-Location $taskRoot
try {
    function Stable {
        & git diff --quiet $Source -- tools/long-run-supervisor-074/src
        if($LASTEXITCODE -ne 0 -or (& git status --porcelain -- tools/long-run-supervisor-074/src) -or (& git rev-parse HEAD).Trim() -cne $Source){throw 'Wrong/dirty source'}
    }
    Stable
    $taskImages=@('/home/artur/.cache/hydracache-074-194-check/release/deps/hydracache_long_run_supervisor_074-b26b156d3dcd0098','/home/artur/.cache/hydracache-074-188-check/release/deps/hydracache_long_run_supervisor_074-d2aba7f58d773234')
    function Images { @($taskImages | ForEach-Object {(& wsl.exe -d Ubuntu -- sha256sum $_).Trim();if($LASTEXITCODE -ne 0){throw 'Missing image'}}) }
    $taskHashes=Images
    $taskBefore=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if($LASTEXITCODE -ne 0 -or $taskBefore -cne 'active'){throw 'Require active local clock service'}
    [ordered]@{source=$Source;images=$taskHashes;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();clock_before=$taskBefore;original_offsets_ns=@(300000000000,60000000000,10000000000);test_issuers=$true;actual_systemd_placement=$false;original_send=$false;completion=$false;server=$false;supervisor=$false;product=$false} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $taskOut 'identity.json') -Encoding utf8NoBOM
    & wsl.exe -d Ubuntu -u root -- systemctl stop systemd-timesyncd.service
    if($LASTEXITCODE -ne 0){throw 'Local clock stop failed'}
    $taskResults=@()
    try {
        & wsl.exe -d Ubuntu -- python3 -u -c "import time,json
start=time.monotonic(); stable=start; prev=time.time_ns(); steps=0; samples=0
while time.monotonic()-start<180:
 now=time.time_ns(); mono=time.monotonic(); samples+=1
 if now<prev: steps+=1; stable=mono
 if mono-stable>=65: print(json.dumps(dict(stable_seconds=65,backward_steps=steps,samples=samples,attempt=False))); break
 prev=now; time.sleep(.01)
else: raise RuntimeError('clock not stable; no attempt')" 2>&1 | Tee-Object -FilePath (Join-Path $taskOut 'clock-stable.log')
        if($LASTEXITCODE -ne 0){throw 'Clock stability failed; no attempt'}
        for($taskIndex=0;$taskIndex -lt 2;$taskIndex++){
            $taskCompiler=if($taskIndex -eq 0){'194'}else{'188'}
            & wsl.exe -d Ubuntu -u root -- env HC074_OWNED_ROOT_FIXTURE=local-only-v1 HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017 $taskImages[$taskIndex] privileged_owned_placement_live_join --ignored --nocapture --test-threads=1 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "live$taskCompiler.log")
            $taskExit=$LASTEXITCODE
            $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskOut "live$taskCompiler.log")
            $taskRoots=@(& wsl.exe -d Ubuntu -u root -- find /sys/fs/cgroup -maxdepth 1 -name 'hc074-owned-root-*' -print)
            $taskScan=$LASTEXITCODE
            $taskRows=[regex]::Matches($taskRaw,'HC074-PLACEMENT-PREPARATION (\{[^\r\n]+\})')
            $taskFacts=@($taskRows | ForEach-Object {$_.Groups[1].Value | ConvertFrom-Json})
            $taskPassed=$taskExit -eq 0 -and $taskRaw.Contains('test result: ok. 1 passed; 0 failed;') -and [regex]::Matches($taskRaw,'HC074-OWNED-PLACEMENT-LIVE case=').Count -eq 7 -and $taskFacts.Count -eq 7 -and @($taskFacts | Where-Object returned -CNE $true).Count -eq 0
            $taskResults+=,[ordered]@{compiler=$taskCompiler;wsl_exit_code=$taskExit;full_cohort_passed=$taskPassed;completed_cases=@($taskFacts | Where-Object returned -CEQ $true | ForEach-Object case);failed_cases=@($taskFacts | Where-Object returned -CNE $true | ForEach-Object case);fixture_scan_exit_code=$taskScan;fixture_roots=$taskRoots}
            $taskResults | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $taskOut 'compiler-outcomes.json') -Encoding utf8NoBOM
            if($taskScan -ne 0 -or $taskRoots.Count -ne 0){throw 'Local fixture roots remain; no next attempt'}
            Stable
            if(($taskHashes -join ',') -cne ((Images) -join ',')){throw 'Image drift'}
        }
    }finally{
        & wsl.exe -d Ubuntu -u root -- systemctl start systemd-timesyncd.service
        if($LASTEXITCODE -ne 0){throw 'Clock restoration failed'}
        $taskAfter=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
        $taskAfter | Set-Content -LiteralPath (Join-Path $taskOut 'clock-restored.log') -Encoding utf8NoBOM
        if($LASTEXITCODE -ne 0 -or $taskAfter -cne 'active'){throw 'Clock restoration unverified'}
    }
    Stable
    $taskPassed=$taskResults.Count -eq 2 -and @($taskResults | Where-Object full_cohort_passed -CNE $true).Count -eq 0
    [ordered]@{source=$Source;local_live_gate_passed=$taskPassed;clock_after=$taskAfter;original_send=$false;server=$false;supervisor=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'outcome.json') -Encoding utf8NoBOM
    if(-not $taskPassed){throw 'Full live matrix remains incomplete; preserve all causal facts'}
}finally{Pop-Location}
