param(
    [Parameter(Mandatory)][ValidatePattern('^[a-z0-9-]+$')][string]$Label,
    [Parameter(Mandatory)][string]$Image,
    [Parameter(Mandatory)][string]$Source,
    [ValidateSet('baseline','candidate','reverted')][string]$Kind='baseline'
)
$ErrorActionPreference='Stop'
$taskRoot=(Resolve-Path (Join-Path $PSScriptRoot '../../../../../..')).Path
$taskOut=Join-Path $PSScriptRoot $Label
if(Test-Path -LiteralPath $taskOut){throw 'Do not overwrite a cohort'}
New-Item -ItemType Directory -Path $taskOut | Out-Null
Push-Location $taskRoot
try {
    if((& git status --porcelain -- tools/long-run-supervisor-074/src tools/long-run-supervisor-074/tests)){throw 'Runtime source dirty'}
    & git diff --quiet $Source -- tools/long-run-supervisor-074/src tools/long-run-supervisor-074/tests
    if($LASTEXITCODE -ne 0){throw 'Runtime differs from selected source'}
    $taskHash=(& wsl.exe -d Ubuntu -- sha256sum $Image).Trim()
    if($LASTEXITCODE -ne 0 -or $taskHash -notmatch '^[0-9a-f]{64}  /'){throw 'Missing image'}
    $taskUid=(& wsl.exe -d Ubuntu -- id -u).Trim()
    if($LASTEXITCODE -ne 0 -or $taskUid -cne '1000'){throw 'Require nonroot'}
    function Run([string]$Name,[string[]]$Arguments,[int]$Passed=-1){
        $taskLog=Join-Path $taskOut "$Name.log"
        if(Test-Path -LiteralPath $taskLog){throw 'Retain previous evidence'}
        & wsl.exe @Arguments 2>&1 | Tee-Object -FilePath $taskLog
        if($LASTEXITCODE -ne 0){throw "Failed: $Name"}
        if($Passed -ge 0 -and -not (Get-Content -Raw -LiteralPath $taskLog).Contains("test result: ok. $Passed passed; 0 failed;")){throw 'Zero or wrong test selection'}
    }
    $taskBefore=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
    if($LASTEXITCODE -ne 0 -or $taskBefore -cne 'active'){throw 'Require active local timesyncd'}
    & wsl.exe -d Ubuntu -u root -- systemctl stop systemd-timesyncd.service
    if($LASTEXITCODE -ne 0){throw 'Stop exact local timesyncd failed'}
    try {
        Run 'clock-settle' @('-d','Ubuntu','--','python3','-u','-c',"import time
start=time.monotonic(); stable=start; prev=time.time_ns(); steps=0
while time.monotonic()-start < 180:
 now=time.time_ns(); mono=time.monotonic()
 if now < prev: steps+=1; stable=mono; print('backward_ns='+str(prev-now),flush=True)
 if mono-stable >= 65: print('settled=true stable_seconds=65 steps='+str(steps)+' attempt=false',flush=True); break
 prev=now; time.sleep(.01)
else: raise RuntimeError('clock did not settle')")
        Run 'clock-probe' @('-d','Ubuntu','--','python3','-u','-c',"import time,pathlib,json
start=time.monotonic(); prev=time.time_ns(); n=0
while time.monotonic()-start < 65:
 now=time.time_ns()
 if now < prev: raise RuntimeError('REALTIME backwards; no attempt')
 prev=now; n+=1; time.sleep(.01)
print(json.dumps(dict(backward_samples=0,samples=n,boot=pathlib.Path('/proc/sys/kernel/random/boot_id').read_text().strip(),boottime_ns=time.clock_gettime_ns(time.CLOCK_BOOTTIME))),flush=True)")
        $taskArgs=@('-d','Ubuntu','--','env','HC074_START_MATERIAL_BUNDLE=/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/diagnostic-start-material-bundle-20261017',$Image)
        foreach($taskRepeat in 1..3){ Run "retirement-$taskRepeat" ($taskArgs+@('original_root_retirement_frame_control','--ignored','--nocapture','--test-threads=1')) 1 }
        Run 'frame-regression' ($taskArgs+@('original_root_guardian_frame_controls','--ignored','--nocapture','--test-threads=1')) 1
    } finally {
        & wsl.exe -d Ubuntu -u root -- systemctl start systemd-timesyncd.service
        if($LASTEXITCODE -ne 0){throw 'Restore local timesyncd failed'}
        $taskAfter=(& wsl.exe -d Ubuntu -- systemctl is-active systemd-timesyncd.service).Trim()
        $taskAfter | Set-Content -LiteralPath (Join-Path $taskOut 'clock-restored.log') -Encoding utf8NoBOM
        if($LASTEXITCODE -ne 0 -or $taskAfter -cne 'active'){throw 'Restore unverified'}
    }
    if($taskHash -cne (& wsl.exe -d Ubuntu -- sha256sum $Image).Trim()){throw 'Image drift'}
    [ordered]@{source=$Source;kind=$Kind;image=$taskHash;uid=$taskUid;retirement_repeats=3;frame_cases=8;clock_before=$taskBefore;clock_after=$taskAfter;original_constructor=$false;server=$false;supervisor=$false;product=$false} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $taskOut 'receipt.json') -Encoding utf8NoBOM
} finally {Pop-Location}
