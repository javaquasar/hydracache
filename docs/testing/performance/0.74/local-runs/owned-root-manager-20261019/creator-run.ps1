param([Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label,[Parameter(Mandatory)][string]$GnuImage,[Parameter(Mandatory)][string]$MsrvImage,[Parameter(Mandatory)][string]$Source)
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
    $taskHashes=@($GnuImage,$MsrvImage) | ForEach-Object {
        $taskHash=(& wsl.exe -d Ubuntu -- sha256sum $_).Trim()
        if($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /'){throw 'Image missing'}
        $taskHash
    }
    function Run([string]$Name,[string[]]$Arguments){
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath (Join-Path $taskOut "$Name.log")
        if($LASTEXITCODE -ne 0){throw "Failed: $Name"}
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
        $taskWslOut=(& wsl.exe -d Ubuntu -- wslpath -u $taskOut).Trim()
        if($LASTEXITCODE -ne 0){throw 'Evidence path conversion failed'}
        foreach($taskCohort in @(@('screen-gnu194',$GnuImage,'owner'),@('matrix-gnu194',$GnuImage,''),@('matrix-gnu188',$MsrvImage,''))){
            $taskArgs=@('-d','Ubuntu','-u','root','--','env','HC074_OWNED_ROOT_FIXTURE=local-only-v1','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017','python3','-B','-u','tools/long-run-supervisor-074/tests/owned_root_creator_sigkill.py','--binary',$taskCohort[1],'--out',"$taskWslOut/$($taskCohort[0])")
            if($taskCohort[2]){$taskArgs+=@('--case',$taskCohort[2])}
            Run $taskCohort[0] $taskArgs
            $taskRaw=Get-Content -Raw -LiteralPath (Join-Path $taskOut "$($taskCohort[0]).log")
            $taskExpected=if($taskCohort[2]){1}else{6}
            if(([regex]::Matches($taskRaw,'HC074-OWNED-CREATOR case=')).Count -ne $taskExpected -or -not $taskRaw.Contains("completed=true cases=$taskExpected")){throw 'Missing actual creator cases'}
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
    [ordered]@{source=$Source;images=$taskHashes;recipe=(Get-FileHash -LiteralPath $PSCommandPath -Algorithm SHA256).Hash.ToLowerInvariant();creator_sigkill_cases=13;clock_before=$taskBefore;clock_after=$taskAfter;test_issuers=$true;original_send=$false;completion=$false;server=$false;supervisor=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
}finally{Pop-Location}
