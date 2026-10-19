param([Parameter(Mandatory)][ValidateSet('baseline','candidate')][string]$Stage,[Parameter(Mandatory)][string]$Source,[Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut=Join-Path $PSScriptRoot $Label
if(Test-Path -LiteralPath $taskOut){throw 'Retain existing cohort'}
New-Item -ItemType Directory -Path $taskOut | Out-Null
Push-Location $taskRoot
try {
    & git diff --quiet $Source -- tools/long-run-supervisor-074/src
    if($LASTEXITCODE -ne 0 -or (& git status --porcelain -- tools/long-run-supervisor-074/src)){throw 'Wrong/dirty runtime source'}
    $taskHead=(& git rev-parse HEAD).Trim()
    $taskImages=@('/home/artur/.cache/hydracache-074-194-check/release/deps/hydracache_long_run_supervisor_074-b26b156d3dcd0098','/home/artur/.cache/hydracache-074-188-check/release/deps/hydracache_long_run_supervisor_074-d2aba7f58d773234')
    $taskHashes=@($taskImages | ForEach-Object {(& wsl.exe -d Ubuntu -- sha256sum $_).Trim();if($LASTEXITCODE -ne 0){throw 'Missing image'}})
    function Run([string]$Name,[string[]]$Arguments){
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "$Name.log")
        if($LASTEXITCODE -ne 0){throw "Local live check refused: $Name"}
    }
    $taskBefore=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if($LASTEXITCODE -ne 0 -or $taskBefore -cne 'active'){throw 'Require active local clock service'}
    & wsl.exe -d Ubuntu -u root -- systemctl stop systemd-timesyncd.service
    if($LASTEXITCODE -ne 0){throw 'Local clock stop failed'}
    try {
        Run 'clock-stable' @('-d','Ubuntu','--','python3','-u','-c',"import time,json
start=time.monotonic(); stable=start; prev=time.time_ns(); steps=0; samples=0
while time.monotonic()-start<180:
 now=time.time_ns(); mono=time.monotonic(); samples+=1
 if now<prev: steps+=1; stable=mono
 if mono-stable>=65: print(json.dumps(dict(stable_seconds=65,backward_steps=steps,samples=samples,attempt=False))); break
 prev=now; time.sleep(.01)
else: raise RuntimeError('clock not stable; no attempt')")
        $taskName=if($Stage -ceq 'baseline'){'privileged_original_root_manager_writer_screen'}else{'privileged_owned_placement_live_join'}
        for($taskIndex=0;$taskIndex -lt 2;$taskIndex++){
            $taskCompiler=if($taskIndex -eq 0){'194'}else{'188'}
            Run "live$taskCompiler" @('-d','Ubuntu','-u','root','--','env','HC074_OWNED_ROOT_FIXTURE=local-only-v1','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017',$taskImages[$taskIndex],$taskName,'--ignored','--nocapture','--test-threads=1')
            $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskOut "live$taskCompiler.log")
            if(-not $taskRaw.Contains('test result: ok. 1 passed; 0 failed;')){throw 'Wrong live selection'}
            $taskExpected=if($Stage -ceq 'baseline'){1}else{7}
            $taskPattern=if($Stage -ceq 'baseline'){'HC074-OWNED-ROOT case=retire'}else{'HC074-OWNED-PLACEMENT-LIVE case='}
            if([regex]::Matches($taskRaw,$taskPattern).Count -ne $taskExpected){throw 'Incomplete live cohort'}
            $taskRoots=@(& wsl.exe -d Ubuntu -u root -- find /sys/fs/cgroup -maxdepth 1 -name 'hc074-owned-root-*' -print)
            if($LASTEXITCODE -ne 0 -or $taskRoots.Count -ne 0){throw 'Local fixture roots remain; stop'}
        }
    }finally{
        & wsl.exe -d Ubuntu -u root -- systemctl start systemd-timesyncd.service
        if($LASTEXITCODE -ne 0){throw 'Clock restoration failed'}
        $taskAfter=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
        $taskAfter | Set-Content -LiteralPath (Join-Path $taskOut 'clock-restored.log') -Encoding utf8NoBOM
        if($taskAfter -cne 'active'){throw 'Clock restoration unverified'}
    }
    if((& git rev-parse HEAD).Trim() -cne $taskHead){throw 'HEAD drift'}
    $taskAgain=@($taskImages | ForEach-Object {(& wsl.exe -d Ubuntu -- sha256sum $_).Trim()})
    if(($taskHashes -join ',') -cne ($taskAgain -join ',')){throw 'Image drift'}
    [ordered]@{source=$Source;stage=$Stage;images=$taskHashes;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();clock_before=$taskBefore;clock_after=$taskAfter;original_offsets_ns=@(300000000000,60000000000,10000000000);test_issuers=$true;actual_systemd_placement=$false;original_send=$false;completion=$false;server=$false;supervisor=$false;product=$false} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
