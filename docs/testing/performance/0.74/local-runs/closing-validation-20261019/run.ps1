param([Parameter(Mandatory)][ValidateSet('cost','remaining')][string]$Stage,[Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label,[Parameter(Mandatory)][string]$GnuImage,[Parameter(Mandatory)][string]$MsrvImage,[Parameter(Mandatory)][string]$Source)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut=Join-Path $PSScriptRoot $Label
if(Test-Path -LiteralPath $taskOut){throw 'Retain existing cohort'}
New-Item -ItemType Directory -Path $taskOut | Out-Null
Push-Location $taskRoot
try {
    if((& git status --porcelain -- tools/long-run-supervisor-074/src tools/long-run-supervisor-074/tests)){throw 'Fixture source dirty'}
    & git diff --quiet $Source -- tools/long-run-supervisor-074/src tools/long-run-supervisor-074/tests
    if($LASTEXITCODE -ne 0){throw 'Wrong selected source'}
    $taskHead=(& git rev-parse HEAD).Trim()
    $taskWslOut=(& wsl.exe -d Ubuntu -- wslpath -u ($taskOut.Replace('\','/')))
    if($LASTEXITCODE -ne 0 -or -not $taskWslOut){throw 'Evidence path conversion failed before attempt'}
    $taskWslOut=$taskWslOut.Trim()
    $taskHashes=@($GnuImage,$MsrvImage) | ForEach-Object {
        $taskHash=(& wsl.exe -d Ubuntu -- sha256sum $_).Trim()
        if($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /'){throw 'Image missing'}
        $taskHash
    }
    function Run([string]$Name,[string[]]$Arguments,[switch]$AllowTimeRefusal){
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "$Name.log")
        $script:taskLastCode=$LASTEXITCODE
        if($script:taskLastCode -ne 0){
            $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskOut "$Name.log")
            if(-not $AllowTimeRefusal -or $taskRaw -notmatch 'Time' -or $taskRaw -notmatch 'test result: FAILED\. 0 passed; 1 failed;' -or $taskRaw -match 'panic in a destructor|During handling of the above exception'){throw "Failed: $Name"}
        }
    }
    $taskBefore=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if($LASTEXITCODE -ne 0 -or $taskBefore -cne 'active'){throw 'Require active local clock service'}
    & wsl.exe -d Ubuntu -u root -- systemctl stop systemd-timesyncd.service
    if($LASTEXITCODE -ne 0){throw 'Local clock stop failed'}
    try {
        Run 'clock-settle' @('-d','Ubuntu','--','python3','-u','-c',"import time
start=time.monotonic(); stable=start; prev=time.time_ns(); steps=0
while time.monotonic()-start<180:
 now=time.time_ns(); mono=time.monotonic()
 if now<prev: steps+=1; stable=mono
 if mono-stable>=65: print('settled=true stable_seconds=65 steps='+str(steps)+' attempt=false'); break
 prev=now; time.sleep(.01)
else: raise RuntimeError('clock not stable')")
        Run 'clock-probe' @('-d','Ubuntu','--','python3','-u','-c',"import time,pathlib,json
start=time.monotonic(); prev=time.time_ns(); n=0
while time.monotonic()-start<65:
 now=time.time_ns()
 if now<prev: raise RuntimeError('REALTIME backwards; no attempt')
 prev=now; n+=1; time.sleep(.01)
print(json.dumps(dict(backward_samples=0,samples=n,boot=pathlib.Path('/proc/sys/kernel/random/boot_id').read_text().strip())))")
        $taskRows=@()
        $taskCohorts=if($Stage -ceq 'cost'){
            @(@('cost-gnu194-0',$GnuImage,'owner'),@('cost-gnu194-1',$GnuImage,'owner'),@('cost-gnu194-2',$GnuImage,'owner'),@('cost-gnu188-0',$MsrvImage,'owner'),@('cost-gnu188-1',$MsrvImage,'owner'),@('cost-gnu188-2',$MsrvImage,'owner'))
        }else{
            @(@('second-dead',$MsrvImage,'second-dead'),@('first-stopped',$MsrvImage,'first-stopped'),@('second-stopped',$MsrvImage,'second-stopped'),@('outside-alias',$MsrvImage,'outside-alias'),@('owner',$MsrvImage,'owner'),@('first-dead',$MsrvImage,'first-dead'))
        }
        $taskMode=if($Stage -ceq 'cost'){'cost'}else{'facts'}
        foreach($taskCohort in $taskCohorts){
            $taskArgs=@('-d','Ubuntu','-u','root','--','env','HC074_OWNED_ROOT_FIXTURE=local-only-v1',"HC074_CLOSING_DIAGNOSTICS=$taskMode",'HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017','python3','-B','-u','tools/long-run-supervisor-074/tests/owned_root_creator_sigkill.py','--binary',$taskCohort[1],'--out',"$taskWslOut/$($taskCohort[0])",'--case',$taskCohort[2])
            Run $taskCohort[0] $taskArgs -AllowTimeRefusal:($Stage -ceq 'cost')
            $taskExit=$script:taskLastCode
            $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskOut "$($taskCohort[0]).log")
            if($taskExit -eq 0 -and (([regex]::Matches($taskRaw,'HC074-OWNED-CREATOR case=')).Count -ne 1 -or -not $taskRaw.Contains('completed=true cases=1'))){throw 'Missing actual creator case'}
            $taskRoots=@(& wsl.exe -d Ubuntu -u root -- find /sys/fs/cgroup -maxdepth 1 -name 'hc074-owned-root-*' -print)
            if($LASTEXITCODE -ne 0 -or $taskRoots.Count -ne 0){throw 'Fixture cleanup not empty; no next attempt'}
            $taskRows+= [ordered]@{label=$taskCohort[0];case=$taskCohort[2];exit_code=$taskExit;actual_creator_sigkill=($taskExit -eq 0);diagnostic_mode=$taskMode}
        }
    }finally{
        & wsl.exe -d Ubuntu -u root -- systemctl start systemd-timesyncd.service
        if($LASTEXITCODE -ne 0){throw 'Clock restoration failed'}
        $taskAfter=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
        $taskAfter | Set-Content -LiteralPath (Join-Path $taskOut 'clock-restored.log') -Encoding utf8NoBOM
        if($LASTEXITCODE -ne 0 -or $taskAfter -cne 'active'){throw 'Clock restoration unverified'}
    }
    if((& git rev-parse HEAD).Trim() -cne $taskHead){throw 'Source drift'}
    $taskAgain=@($GnuImage,$MsrvImage) | ForEach-Object {(& wsl.exe -d Ubuntu -- sha256sum $_).Trim()}
    if(($taskHashes -join "`n") -cne ($taskAgain -join "`n")){throw 'Image drift'}
    [ordered]@{source=$Source;images=$taskHashes;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();stage=$Stage;rows=$taskRows;functional_gate_passed=($Stage -ceq 'remaining');not_a_combined_cross_source_matrix=$true;clock_before=$taskBefore;clock_after=$taskAfter;test_issuers=$true;original_send=$false;completion=$false;server=$false;supervisor=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
