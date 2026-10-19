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
        for($taskIndex=0;$taskIndex -lt 3;$taskIndex++){
            $taskUsage="/mnt/c/Workspace/prj/jq/cashe/hydracache-074/docs/testing/performance/0.74/local-runs/placement-stage-cost-20261020/$Label/usage-$taskIndex.log"
            & wsl.exe -d Ubuntu -u root -- env HC074_OWNED_ROOT_FIXTURE=local-only-v1 HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017 /usr/bin/time -v -o $taskUsage $taskImages[0] privileged_placement_preparation_cost --ignored --nocapture --test-threads=1 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "cost-$taskIndex.log")
            $taskExit=$LASTEXITCODE
            $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskOut "cost-$taskIndex.log")
            $taskRoots=@(& wsl.exe -d Ubuntu -u root -- find /sys/fs/cgroup -maxdepth 1 -name 'hc074-owned-root-*' -print)
            $taskScan=$LASTEXITCODE
            $taskRows=[regex]::Matches($taskRaw,'HC074-PLACEMENT-COST (\{[^\r\n]+\})')
            if($taskRows.Count -ne 1){throw 'Missing or duplicate original cost observation'}
            $taskFacts=$taskRows[0].Groups[1].Value | ConvertFrom-Json
            $taskResults+=,[ordered]@{attempt=$taskIndex;wsl_exit_code=$taskExit;returned=$taskFacts.returned;healthy_gate=$false;fixture_scan_exit_code=$taskScan;fixture_roots=$taskRoots}
            $taskResults | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $taskOut 'attempt-outcomes.json') -Encoding utf8NoBOM
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
    if($taskResults.Count -ne 3){throw 'Incomplete preregistered cost cohort'}
    [ordered]@{source=$Source;cost_observations=$taskResults.Count;local_live_gate_passed=$false;healthy_gate=$false;clock_after=$taskAfter;original_send=$false;server=$false;supervisor=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'outcome.json') -Encoding utf8NoBOM
}finally{Pop-Location}
